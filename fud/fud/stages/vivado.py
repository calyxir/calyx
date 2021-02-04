from pathlib import Path
import shutil
import re

from fud.stages import SourceType, Stage

from ..vivado.extract import futil_extract
from .remote_context import RemoteExecution
from ..utils import TmpDir, shell


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
        # @self.step(input_type=SourceType.Directory)
        # def run_vivado(tmpdir):
        #     proc = shell(
        #         " ".join([f"cd {tmpdir.name}", "&&", self.cmd]), wait=False

        @self.step(description=self.cmd)
        def run_vivado(tmpdir: SourceType.Directory):
            shell(" ".join([f"cd {tmpdir.name}", "&&", self.cmd]), stdout_as_debug=True)
            # for chunk in iter(lambda: proc.stdout.readline(2048), ""):
            #     if proc.poll() is not None:
            #         break
            #     chunk = chunk.decode("ascii").strip()
            #     r = re.search(r"Phase (\d(?:\d|\.)*)", chunk)
            #     if r is not None:
            #         step.spinner.start_step(f"{step.name} ({chunk})")

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
        @self.step()
        def extract(directory: SourceType.Directory) -> SourceType.String:
            """
            Extract relevant data from Vivado synthesis files.
            """
            return futil_extract(Path(directory.name))

        return extract(input_dir)
