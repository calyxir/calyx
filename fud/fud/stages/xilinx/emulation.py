import logging as log

from pathlib import Path

from fud.stages import Stage, SourceType, Source
from fud import errors
from fud.stages.remote_context import RemoteExecution, LocalSandbox
from fud.utils import shell


class HwEmulationStage(Stage):
    name = "wdb"

    def __init__(self, config):
        super().__init__(
            src_state="xclbin",
            target_state="wdb",
            input_type=SourceType.Path,
            output_type=SourceType.Path,
            config=config,
            description="Runs Vivado hardware emulation",
        )

        self.xilinx_location = self.config["stages", self.name, "xilinx_location"]
        self.xrt_location = self.config["stages", self.name, "xrt_location"]
        self.setup_commands = (
            f"source {self.xilinx_location}/settings64.sh && "
            f"source {self.xrt_location}/setup.sh"
        )

        self.host_cpp = self.config["stages", self.name, "host"]
        self.save_temps = bool(self.config["stages", self.name, "save_temps"])

        self.xrt = (
            Path(self.config["global", "futil_directory"])
            / "fud"
            / "bitstream"
            / "xrt.ini"
        )
        self.sim_script = (
            Path(self.config["global", "futil_directory"])
            / "fud"
            / "bitstream"
            / "sim_script.tcl"
        )
        self.mode = self.config["stages", self.name, "mode"]
        self.device = "xilinx_u50_gen3x16_xdma_201920_3"  # TODO: Hard-coded.

        # remote execution
        self.remote_exec = RemoteExecution(self)
        self.temp_location = self.config["stages", "xclbin", "temp_location"]

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

    def _define_steps(self, input_data):
        @self.step()
        def check_host_cpp():
            """
            Make sure that `-s wdb.host` is provided
            """
            if self.host_cpp is None:
                raise errors.MissingDynamicConfiguration("wdb.host")

        @self.step()
        def compile_host(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Compile the host code
            """
            cmd = (
                f"cd {tmpdir} && "
                "g++ "
                f"-I{self.xrt_location}/include "
                f"-I{self.xilinx_location}/include "
                "-Wall -O0 -g -std=c++14 -fmessage-length=0 "
                "host.cpp "
                "-o 'host' "
                f"-L{self.xrt_location}/lib -lOpenCL -lpthread -lrt -lstdc++"
            )
            self._shell(client, cmd)

        @self.step()
        def generate_emconfig(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Generate emconfig.json
            """
            cmd = (
                f"cd {tmpdir} && "
                f"{self.xilinx_location}/bin/emconfigutil "
                f"--platform {self.device} "
                "--od ."
            )
            self._shell(client, cmd)

        @self.step()
        def emulate(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Emulation the xclbin
            """
            cmd = (
                f"cd {tmpdir} && {self.setup_commands} && "
                f"XCL_EMULATION_MODE={self.mode} "
                f"./host kernel.xclbin {self.device}"
            )
            self._shell(client, cmd)

        check_host_cpp()

        file_map = {
            input_data: "kernel.xclbin",
            self.host_cpp: "host.cpp",
            self.xrt: "xrt.ini",
            self.sim_script: "sim_script.tcl",
        }
        if self.remote_exec.use_ssh:
            self.remote_exec.import_libs()
            client, tmpdir = self.remote_exec.open_and_send(file_map)
        else:
            sandbox = LocalSandbox(self, self.save_temps)
            tmpdir = sandbox.create(file_map)
            client = Source(None, SourceType.UnTyped)

        compile_host(client, tmpdir)
        generate_emconfig(client, tmpdir)
        emulate(client, tmpdir)

        wdb_name = f"{self.device}-0-kernel.wdb"
        if self.remote_exec.use_ssh:
            return self.remote_exec.close_and_get(
                client,
                tmpdir,
                wdb_name,
                keep=self.save_temps,
            )
        else:
            return sandbox.get_file(wdb_name)
