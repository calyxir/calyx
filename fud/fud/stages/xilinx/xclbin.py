import logging as log
from pathlib import Path
import shutil

from fud.stages import Source, SourceType, Stage
from fud.stages.remote_context import RemoteExecution
from fud.stages.futil import FutilStage
from fud.utils import TmpDir, shell


class XilinxStage(Stage):
    name = "xclbin"

    def __init__(self, config):
        super().__init__(
            src_state="futil",
            target_state="xclbin",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            config=config,
            description="compiles Calyx programs to Xilinx bitstreams",
        )

        # tcl files
        self.gen_xo_tcl = (
            Path(self.config["global", "futil_directory"])
            / "fud"
            / "bitstream"
            / "gen_xo.tcl"
        )

        # sub stages to use futil to compile
        self.xilinx_futil = FutilStage(config, "xilinx-verilog", "-b xilinx", "")
        self.xml_futil = FutilStage(config, "xilinx-verilog", "-b xilinx-xml", "")
        self.kernel_futil = FutilStage(
            config, "xilinx-verilog", "-b verilog --synthesis -p external", ""
        )

        # remote execution
        self.remote_exec = RemoteExecution(self)
        self.temp_location = self.config["stages", self.name, "temp_location"]

        self.mode = self.config["stages", self.name, "mode"]
        self.device = self.config["stages", self.name, "device"]

        self.setup()

    def _shell(self, client, cmd):
        """Run a command, either locally or remotely."""
        if self.remote_exec.use_ssh:
            _, stdout, stderr = client.exec_command(cmd)
            for chunk in iter(lambda: stdout.readline(2048), ""):
                log.debug(chunk.strip())
            log.debug(stderr.read().decode("UTF-8").strip())

        else:
            stdout = shell(cmd, capture_stdout=False)
            log.debug(stdout)

    def _sandbox(self, input_files):
        """Copy input files to a fresh temporary directory.

        `input_files` is a dict with the same format as `open_and_send`:
        it maps local Source paths to destination strings.

        Return a path to the newly-created temporary directory.
        """

        @self.step()
        def copy_file(
            tmpdir: SourceType.String,
            src_path: SourceType.Path,
            dest_path: SourceType.String,
        ):
            """Copy an input file."""
            shutil.copyfile(src_path, Path(tmpdir) / dest_path)

        tmpdir = Source(TmpDir(), SourceType.Directory)
        for src_path, dest_path in input_files.items():
            if not isinstance(src_path, Source):
                src_path = Source(src_path, SourceType.Path)
            if not isinstance(dest_path, Source):
                dest_path = Source(dest_path, SourceType.String)
            copy_file(tmpdir, src_path, dest_path)
        return tmpdir

    def _define_steps(self, input_data):
        # Step 1: Make a new temporary directory
        @self.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store generated files.
            """
            return TmpDir()

        # Step 2: Compile input using `-b xilinx`
        @self.step()
        def compile_xilinx(inp: SourceType.Stream) -> SourceType.Path:
            """
            Generate AXI controller to interface with the Calyx kernel.
            """
            return (
                self.xilinx_futil.run(Source(inp, SourceType.Stream))
                .convert_to(SourceType.Path)
                .data
            )

        # Step 3: Compiler input using `-b xilinx-xml`
        @self.step()
        def compile_xml(inp: SourceType.Stream) -> SourceType.Path:
            """
            Generate XML configuration from Calyx input.
            """
            return (
                self.xml_futil.run(Source(inp, SourceType.Stream))
                .convert_to(SourceType.Path)
                .data
            )

        # Step 3: Compiler input using `-b xilinx-xml`
        @self.step()
        def compile_kernel(inp: SourceType.Stream) -> SourceType.Path:
            """
            Compile Calyx program to synthesizable Verilog for Xilinx tools.
            """
            return (
                self.kernel_futil.run(Source(inp, SourceType.Stream))
                .convert_to(SourceType.Path)
                .data
            )

        @self.step()
        def package_xo(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Package verilog into XO file.
            """
            cmd = " ".join(
                [
                    f"cd {tmpdir}",
                    "&&",
                    "mkdir -p xclbin",
                    "&&",
                    "/scratch/opt/Xilinx/Vivado/2020.2/bin/vivado",
                    "-mode batch",
                    "-source gen_xo.tcl",
                    f"-tclargs xclbin/kernel.xo kernel {self.mode} {self.device}",
                ]
            )
            self._shell(client, cmd)

        @self.step()
        def compile_xclbin(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Compile XO into xclbin.
            """
            cmd = " ".join(
                [
                    f"cd {tmpdir}",
                    "&&",
                    "/scratch/opt/Xilinx/Vitis/2020.2/bin/v++ -g",
                    f"-t {self.mode}",
                    f"--platform {self.device}",
                    "--save-temps",
                    "--profile.data all:all:all",
                    "--profile.exec all:all:all",
                    "-lo xclbin/kernel.xclbin",
                    "xclbin/kernel.xo",
                ]
            )
            self._shell(client, cmd)

        @self.step()
        def read_file(
            tmpdir: SourceType.Directory,
            name: SourceType.String,
        ) -> SourceType.Stream:
            """Read an output file."""
            return Path(tmpdir.name) / name.data

        if self.remote_exec.use_ssh:
            self.remote_exec.import_libs()

        xilinx = compile_xilinx(input_data)
        xml = compile_xml(input_data)
        kernel = compile_kernel(input_data)

        file_map = {
            xilinx: "toplevel.v",
            kernel: "main.sv",
            xml: "kernel.xml",
            self.gen_xo_tcl: "gen_xo.tcl",
        }
        if self.remote_exec.use_ssh:
            client, tmpdir = self.remote_exec.open_and_send(file_map)
        else:
            tmpdir = self._sandbox(file_map)
            client = Source(None, SourceType.UnTyped)

        package_xo(client, tmpdir)
        compile_xclbin(client, tmpdir)

        if self.remote_exec.use_ssh:
            xclbin = self.remote_exec.close_and_get(
                client,
                tmpdir,
                "xclbin/kernel.xclbin",
            )
        else:
            xclbin = read_file(
                tmpdir,
                Source("xclbin/kernel.xclbin", SourceType.String),
            )
        return xclbin
