from pathlib import Path
import shutil

from fud.stages import SourceType, Stage

from ..vivado.extract import futil_extract
from ..vivado.stage_template import RemoteExecution
from ..utils import TmpDir


class VivadoStage(Stage):
    def __init__(self, config):
        super().__init__(
            "synth-verilog",
            "synth-files",
            SourceType.Path,
            SourceType.Directory,
            config,
            "Runs synthesis on a Verilog program",
        )
        self.device_files = [
            str(
                Path(self.config["global", "futil_directory"])
                / "fud"
                / "synth"
                / "synth.tcl"
            ),
            str(
                Path(self.config["global", "futil_directory"])
                / "fud"
                / "synth"
                / "device.xdc"
            ),
        ]
        self.target_name = "main.sv"
        self.remote_context = RemoteExecution(self)
        self.use_ssh = self.remote_context.use_ssh
        self.cmd = "vivado -mode batch -source synth.tcl"
        self.setup()

    def _define_steps(self, verilog_path):
        local_tmpdir = self.setup_environment(verilog_path)
        if self.use_ssh:
            (client, remote_tmpdir) = self.remote_context.open_and_transfer(
                verilog_path
            )
            self.remote_context.execute(client, remote_tmpdir, self.cmd)
            self.remote_context.close_and_transfer(client, remote_tmpdir, local_tmpdir)
        else:
            self.execute(local_tmpdir)
        return local_tmpdir

    def setup_environment(self, verilog_path):
        # Step 1: Make a new temporary directory
        @self.step(input_type=SourceType.Null, output_type=SourceType.Directory)
        def mktmp(step):
            """
            Make temporary directory to store Vivado synthesis files.
            """
            return TmpDir()

        @self.step(input_type=(SourceType.Path, SourceType.Directory))
        def local_move_files(step, verilog_path, tmpdir):
            for f in self.device_files:
                shutil.copy(f, tmpdir.name)
            shutil.copy(verilog_path, f"{tmpdir.name}/{self.target_name}")

        tmpdir = mktmp()
        local_move_files(verilog_path, tmpdir)
        return tmpdir

    def execute(self, tmpdir):
        @self.step(input_type=SourceType.Directory)
        def run_vivado(step, tmpdir):
            step.shell(
                " ".join([f"cd {tmpdir.name}", "&&", self.cmd]), stdout_as_debug=True
            )

        run_vivado(tmpdir)

    # TODO cleanup step


class VivadoExtractStage(Stage):
    def __init__(self, config):
        super().__init__(
            "synth-files",
            "resource-estimate",
            SourceType.Directory,
            SourceType.String,
            config,
            "Runs synthesis on a Verilog program",
        )
        self.setup()

    def _define_steps(self, input_dir):
        @self.step(input_type=SourceType.Directory, output_type=SourceType.String)
        def extract(step, directory):
            """
            Extract relevant data from Vivado synthesis files.
            """
            return futil_extract(Path(directory.name))

        return extract(input_dir)
