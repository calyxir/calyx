import shutil
from pathlib import Path, PurePath
import os

from fud.stages import SourceType, Stage
from fud.stages.remote_context import RemoteExecution
from fud.utils import TmpDir, shell
from fud import config as cfg

from .extract import hls_extract, place_and_route_extract


class VivadoBaseStage(Stage):
    """
    Base stage that defines the common steps between
    the Vivado and VivadoHLS.
    """

    def __init__(
        self,
        source,
        destination,
        description,
        target_name=None,
        remote_exec=None,
        flags="",
    ):
        super().__init__(
            src_state=source,
            target_state=destination,
            input_type=SourceType.Path,
            output_type=SourceType.Directory,
            description=description,
        )
        self.target_name = target_name
        self.flags = flags
        self.remote_exec = remote_exec

    def device_files(self, config):
        """
        Device files required for executing this Vivado flow
        """
        pass

    def extra_flags(self, config):
        """
        Extra flags to append to the command.
        """
        return ""

    def _define_steps(self, verilog_path, builder, config):
        use_ssh = bool(config.get(["stages", self.name, "remote"]))
        flags = f"{self.flags} {self.extra_flags(config)}"
        if use_ssh:
            cmd = f"{config['stages', self.name, 'exec']} {flags}"
        else:
            cmd = f"{self.remote_exec} {flags}"

        # Steps and schedule
        local_tmpdir = self.setup_environment(verilog_path, builder, config)
        if use_ssh:
            remote_exec = RemoteExecution(builder, self, config)
            remote_exec.import_libs()
            client, remote_tmpdir = remote_exec.open_and_send(
                {
                    verilog_path: self.target_name,
                    **{p: os.path.basename(p) for p in self.device_files(config)},
                }
            )
            remote_exec.execute(client, remote_tmpdir, cmd)
            remote_exec.close_and_transfer(client, remote_tmpdir, local_tmpdir)
        else:
            VivadoBaseStage.execute(builder, local_tmpdir, cmd)

        return local_tmpdir

    def setup_environment(self, input, builder, config):
        # Step 1: Make a new temporary directory
        @builder.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Vivado synthesis files.
            """
            if ["stages", self.name, "tmpdir"] in config:
                return TmpDir(config["stages", self.name, "tmpdir"])
            else:
                return TmpDir()

        @builder.step()
        def local_move_files(
            verilog_path: SourceType.Path, tmpdir: SourceType.Directory
        ):
            """
            Copy device files into tmpdir.
            """
            for f in self.device_files(config):
                shutil.copy(f, tmpdir.name)
            shutil.copy(str(verilog_path), f"{tmpdir.name}/{self.target_name}")

        tmpdir = mktmp()
        local_move_files(input, tmpdir)
        return tmpdir

    @staticmethod
    def execute(builder, tmpdir, cmd):
        @builder.step(description=cmd)
        def run_vivado(tmpdir: SourceType.Directory):
            shell(" ".join([f"cd {tmpdir.name}", "&&", cmd]), stdout_as_debug=True)

        run_vivado(tmpdir)


class VivadoStage(VivadoBaseStage):
    name = "synth-verilog"

    def __init__(self):
        super().__init__(
            "synth-verilog",
            "synth-files",
            "Produces synthesis files from a Verilog program",
            target_name="main.sv",
            remote_exec="vivado",
            flags="-mode batch -source synth.tcl",
        )

    def device_files(self, config):
        root = Path(config["global", cfg.ROOT])
        # Load constraints
        constraints = config.get(["stages", self.name, "constraints"])
        if constraints:
            constraints = Path(constraints)
        else:
            constraints = root / "fud" / "synth" / "device.xdc"
        # Load synthesis TCL file
        synth = config.get(["stages", self.name, "tcl"])
        if synth:
            tcl = Path(synth)
        else:
            tcl = root / "fud" / "synth" / "synth.tcl"
        return [tcl, constraints]


class VivadoHLSStage(VivadoBaseStage):
    name = "vivado-hls"

    def __init__(self):
        super().__init__(
            "vivado-hls",
            "hls-files",
            "Produces synthesis files from a Vivado C++ program",
            target_name="kernel.cpp",
            remote_exec="vivado_hls",
            flags="-f hls.tcl",
        )

    def device_files(self, config):
        root = Path(config["global", cfg.ROOT])
        return [
            root / "fud" / "synth" / "hls.tcl",
            root / "fud" / "synth" / "fxp_sqrt.h",
        ]

    def extra_flags(self, config):
        top = config.get(["stages", self.name, "top"])
        return f"-tclargs top {top}" if top else ""


class VivadoHLSPlaceAndRouteStage(VivadoBaseStage):
    name = "vivado-hls"

    def __init__(self):
        super().__init__(
            "vivado-hls",
            "hls-files-routed",
            "Performs placement and routing of RTL generated by Vivado HLS",
            target_name="kernel.cpp",
            remote_exec="vivado_hls",
            flags="-f hls.tcl -tclargs impl",
        )

    def device_files(self, config):
        root = Path(config["global", cfg.ROOT])
        return [
            root / "fud" / "synth" / "hls.tcl",
            root / "fud" / "synth" / "fxp_sqrt.h",
        ]

    def extra_flags(self, config):
        top = config.get(["stages", self.name, "top"])
        return f"top {top}" if top else ""


class VivadoExtractStage(Stage):
    name = "synth-files"

    def __init__(self):
        super().__init__(
            src_state="synth-files",
            target_state="resource-estimate",
            input_type=SourceType.Directory,
            output_type=SourceType.String,
            description="Extracts information from Vivado synthesis files",
        )

    def _define_steps(self, input, builder, config):
        @builder.step()
        def extract(directory: SourceType.Directory) -> SourceType.String:
            """
            Extract relevant data from Vivado synthesis files.
            """
            return place_and_route_extract(
                Path(directory.name),
                "FutilBuild.runs",
                PurePath("impl_1", "main_utilization_placed.rpt"),
                PurePath("impl_1", "main_timing_summary_routed.rpt"),
                PurePath("synth_1", "runme.log"),
            )

        return extract(input)


class VivadoHLSExtractStage(Stage):
    name = "hls-files"

    def __init__(self):
        super().__init__(
            src_state="hls-files",
            target_state="hls-estimate",
            input_type=SourceType.Directory,
            output_type=SourceType.String,
            description="Extracts information from Vivado HLS synthesis files",
        )

    def _define_steps(self, input, builder, config):
        @builder.step()
        def extract(directory: SourceType.Directory) -> SourceType.String:
            """
            Extract relevant data from Vivado synthesis files.
            """
            top = config.get(["stages", self.name, "top"]) or "kernel"
            return hls_extract(Path(directory.name), top)

        return extract(input)


class VivadoHLSPlaceAndRouteExtractStage(Stage):
    name = "hls-files-routed"

    def __init__(self):
        super().__init__(
            src_state="hls-files-routed",
            target_state="hls-detailed-estimate",
            input_type=SourceType.Directory,
            output_type=SourceType.String,
            description="Extracts information from Vivado HLS synthesis files",
        )

    def _define_steps(self, input, builder, config):
        @builder.step()
        def extract(directory: SourceType.Directory) -> SourceType.String:
            """
            Extract relevant data from Vivado synthesis files.
            """
            top = config.get(["stages", self.name, "top"]) or "kernel"
            verilog_dir = PurePath("solution1", "impl", "verilog")

            return place_and_route_extract(
                Path(directory.name),
                "benchmark.prj",
                verilog_dir / "report" / f"{top}_utilization_routed.rpt",
                verilog_dir / "report" / f"{top}_timing_routed.rpt",
                verilog_dir / "project.runs" / "bd_0_hls_inst_0_synth_1" / "runme.log",
            )

        return extract(input)
