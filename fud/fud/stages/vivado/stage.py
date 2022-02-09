import shutil
from pathlib import Path
import os

from fud.stages import SourceType, Stage
from fud.stages.remote_context import RemoteExecution
from fud.utils import TmpDir, shell

from .extract import futil_extract, hls_extract


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
        Device files requires for executing this Vivado flow
        """
        pass

    def _define_steps(self, verilog_path, builder, config):
        use_ssh = bool(config.get(["stages", self.name, "remote"]))
        if use_ssh:
            cmd = f"{config['stages', self.name, 'exec']} {self.flags}"
        else:
            cmd = f"{self.remote_exec} {self.flags}"

        # Steps and schedule
        local_tmpdir = self.setup_environment(verilog_path, config)
        if use_ssh:
            remote_exec = RemoteExecution(builder, self, config)
            remote_exec.import_libs()
            client, remote_tmpdir = remote_exec.open_and_send(
                {
                    verilog_path: self.target_name,
                    **{p: os.path.basename(p) for p in self.device_files()},
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
        return [
            Path(config["global", "futil_directory"]) / "fud" / "synth" / "synth.tcl",
            Path(config["global", "futil_directory"]) / "fud" / "synth" / "device.xdc",
        ]


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
        return [
            str(
                Path(config["global", "futil_directory"]) / "fud" / "synth" / "hls.tcl"
            ),
            str(
                Path(config["global", "futil_directory"])
                / "fud"
                / "synth"
                / "fxp_sqrt.h"
            ),
        ]


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
            return futil_extract(Path(directory.name))

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
            return hls_extract(Path(directory.name))

        return extract(input)
