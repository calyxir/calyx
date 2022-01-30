import logging as log
from pathlib import Path

from fud.stages import Source, SourceType, Stage
from fud.stages.remote_context import RemoteExecution, LocalSandbox
from fud.stages.futil import FutilStage
from fud.utils import shell


class XilinxStage(Stage):
    name = "xclbin"

    def __init__(self):
        super().__init__(
            src_state="futil",
            target_state="xclbin",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description="compiles Calyx programs to Xilinx bitstreams",
        )

        # sub stages to use futil to compile
        self.xilinx_futil = FutilStage("xilinx-verilog", "-b xilinx", "")
        self.xml_futil = FutilStage("xilinx-verilog", "-b xilinx-xml", "")
        self.kernel_futil = FutilStage(
            "xilinx-verilog", "-b verilog --synthesis -p external", ""
        )

    def _shell(self, client, cmd, remote_exec):
        """Run a command, either locally or remotely."""
        if remote_exec.use_ssh:
            _, stdout, stderr = client.exec_command(cmd)
            for chunk in iter(lambda: stdout.readline(2048), ""):
                log.debug(chunk.strip())
            log.debug(stderr.read().decode("UTF-8").strip())

        else:
            stdout = shell(cmd, capture_stdout=False)
            log.debug(stdout)

    def _define_steps(self, input_data, builder, config):
        # As a debugging aid, the pass can optionally preserve the
        # (local or remote) sandbox where the Xilinx commands ran.
        save_temps = bool(config["stages", self.name, "save_temps"])

        mode = config["stages", self.name, "mode"]
        device = config["stages", self.name, "device"]

        # remote execution context
        remote_exec = RemoteExecution(self, config)

        # tcl files
        self.gen_xo_tcl = (
            Path(config["global", "futil_directory"])
            / "fud"
            / "bitstream"
            / "gen_xo.tcl"
        )

        # Step 2: Compile input using `-b xilinx`
        @builder.step()
        def compile_xilinx(inp: SourceType.Stream) -> SourceType.Path:
            """
            Generate AXI controller to interface with the Calyx kernel.
            """
            return (
                # XXX(rachit): This will no longer work. Need to call setup on
                # the stage before `run`.
                self.xilinx_futil.run(Source(inp, SourceType.Stream))
                .convert_to(SourceType.Path)
                .data
            )

        # Step 3: Compiler input using `-b xilinx-xml`
        @builder.step()
        def compile_xml(inp: SourceType.Stream) -> SourceType.Path:
            """
            Generate XML configuration from Calyx input.
            """
            return (
                # XXX(rachit): This will no longer work. Need to call setup on
                # the stage before `run`.
                self.xml_futil.run(Source(inp, SourceType.Stream))
                .convert_to(SourceType.Path)
                .data
            )

        # Step 3: Compiler input using `-b xilinx-xml`
        @builder.step()
        def compile_kernel(inp: SourceType.Stream) -> SourceType.Path:
            """
            Compile Calyx program to synthesizable Verilog for Xilinx tools.
            """
            return (
                self.kernel_futil.run(Source(inp, SourceType.Stream))
                .convert_to(SourceType.Path)
                .data
            )

        @builder.step()
        def package_xo(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Package verilog into XO file.
            """
            cmd = (
                f"cd {tmpdir} && "
                "mkdir -p xclbin && "
                "/scratch/opt/Xilinx/Vivado/2020.2/bin/vivado "
                "-mode batch "
                "-source gen_xo.tcl "
                f"-tclargs xclbin/kernel.xo"
            )
            self._shell(client, cmd, remote_exec)

        @builder.step()
        def compile_xclbin(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Compile XO into xclbin.
            """
            cmd = (
                f"cd {tmpdir} && "
                "/scratch/opt/Xilinx/Vitis/2020.2/bin/v++ -g "
                f"-t {mode} "
                f"--platform {device} "
                "--save-temps "
                "--profile.data all:all:all "
                "--profile.exec all:all:all "
                "-lo xclbin/kernel.xclbin "
                "xclbin/kernel.xo"
            )
            self._shell(client, cmd, remote_exec)

        # Schedule
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
            sandbox = LocalSandbox(self, save_temps)
            tmpdir = sandbox.create(file_map)
            client = Source(None, SourceType.UnTyped)

        package_xo(client, tmpdir)
        compile_xclbin(client, tmpdir)

        if self.remote_exec.use_ssh:
            return self.remote_exec.close_and_get(
                client,
                tmpdir,
                "xclbin/kernel.xclbin",
                keep_tmpdir=save_temps,
            )
        else:
            return sandbox.get_file("xclbin/kernel.xclbin")
