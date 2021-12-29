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
        config,
        description,
        device_files=None,
        target_name=None,
        local_exec=None,
        remote_exec=None,
        flags="",
    ):
        super().__init__(
            src_state=source,
            target_state=destination,
            input_type=SourceType.Path,
            output_type=SourceType.Directory,
            config=config,
            description=description,
        )
        self.device_files = device_files
        self.target_name = target_name
        self.remote_exec = RemoteExecution(self)
        self.use_ssh = self.remote_exec.use_ssh
        if self.use_ssh:
            self.cmd = remote_exec + " " + flags
        else:
            self.cmd = local_exec + " " + flags
        self.setup()

    def _define_steps(self, verilog_path):
        local_tmpdir = self.setup_environment(verilog_path)
        if self.use_ssh:
            self.remote_exec.import_libs()
            client, remote_tmpdir = self.remote_exec.open_and_send(
                {
                    verilog_path: self.target_name,
                    **{p: os.path.basename(p) for p in self.device_files},
                }
            )
            self.remote_exec.execute(client, remote_tmpdir, self.cmd)
            self.remote_exec.close_and_transfer(client, remote_tmpdir, local_tmpdir)
        else:
            self.execute(local_tmpdir)
        return local_tmpdir

    def setup_environment(self, verilog_path):
        # Step 1: Make a new temporary directory
        @self.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Vivado synthesis files.
            """
            return TmpDir()

        @self.step()
        def local_move_files(
            verilog_path: SourceType.Path, tmpdir: SourceType.Directory
        ):
            """
            Copy device files into tmpdir.
            """
            for f in self.device_files:
                shutil.copy(f, tmpdir.name)
            shutil.copy(str(verilog_path), f"{tmpdir.name}/{self.target_name}")

        tmpdir = mktmp()
        local_move_files(verilog_path, tmpdir)
        return tmpdir

    def execute(self, tmpdir):
        @self.step(description=self.cmd)
        def run_vivado(tmpdir: SourceType.Directory):
            shell(" ".join([f"cd {tmpdir.name}", "&&", self.cmd]), stdout_as_debug=True)

        run_vivado(tmpdir)


class VivadoStage(VivadoBaseStage):
    name = "synth-verilog"

    def __init__(self, config):
        super().__init__(
            "synth-verilog",
            "synth-files",
            config,
            "Produces synthesis files from a Verilog program",
            device_files=[
                Path(config["global", "futil_directory"])
                / "fud"
                / "synth"
                / "synth.tcl",
                Path(config["global", "futil_directory"])
                / "fud"
                / "synth"
                / "device.xdc",
            ],
            target_name="main.sv",
            local_exec=config["stages", self.name, "exec"],
            remote_exec="vivado",
            flags="-mode batch -source synth.tcl",
        )


class VivadoHLSStage(VivadoBaseStage):
    name = "vivado-hls"

    def __init__(self, config):
        super().__init__(
            "vivado-hls",
            "hls-files",
            config,
            "Produces synthesis files from a Vivado C++ program",
            device_files=[
                str(
                    Path(config["global", "futil_directory"])
                    / "fud"
                    / "synth"
                    / "hls.tcl"
                ),
                str(
                    Path(config["global", "futil_directory"])
                    / "fud"
                    / "synth"
                    / "fxp_sqrt.h"
                ),
            ],
            target_name="kernel.cpp",
            local_exec=config["stages", self.name, "exec"],
            remote_exec="vivado_hls",
            flags="-f hls.tcl",
        )


class VivadoExtractStage(Stage):
    name = "synth-files"

    def __init__(self, config):
        super().__init__(
            src_state="synth-files",
            target_state="resource-estimate",
            input_type=SourceType.Directory,
            output_type=SourceType.String,
            config=config,
            description="Extracts information from Vivado synthesis files",
        )
        self.setup()

    def _define_steps(self, input_dir):
        @self.step()
        def extract(directory: SourceType.Directory) -> SourceType.String:
            """
            Extract relevant data from Vivado synthesis files.
            """
            return futil_extract(Path(directory.name))

        return extract(input_dir)


class VivadoHLSExtractStage(Stage):
    name = "hls-files"

    def __init__(self, config):
        super().__init__(
            src_state="hls-files",
            target_state="hls-estimate",
            input_type=SourceType.Directory,
            output_type=SourceType.String,
            config=config,
            description="Extracts information from Vivado HLS synthesis files",
        )
        self.setup()

    def _define_steps(self, input_dir):
        @self.step()
        def extract(directory: SourceType.Directory) -> SourceType.String:
            """
            Extract relevant data from Vivado synthesis files.
            """
            return hls_extract(Path(directory.name))

        return extract(input_dir)
