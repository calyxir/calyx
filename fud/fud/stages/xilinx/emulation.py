import logging as log

from pathlib import Path

from fud.stages import Stage, SourceType, Source
from fud import errors
from fud.stages.remote_context import RemoteExecution, LocalSandbox
from fud.utils import shell


class HwEmulationStage(Stage):
    name = "wdb"

    def __init__(self):
        super().__init__(
            src_state="xclbin",
            target_state="wdb",
            input_type=SourceType.Path,
            output_type=SourceType.Path,
            description="Runs Vivado hardware emulation",
        )

        self.device = "xilinx_u50_gen3x16_xdma_201920_3"  # TODO: Hard-coded.

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

    def _define_steps(self, builder, config):

        xilinx_location = config["stages", self.name, "xilinx_location"]
        xrt_location = config["stages", self.name, "xrt_location"]
        setup_commands = (
            f"source {xilinx_location}/settings64.sh && "
            f"source {xrt_location}/setup.sh"
        )

        host_cpp = config["stages", self.name, "host"]
        save_temps = bool(config["stages", self.name, "save_temps"])
        xrt = (
            Path(config["global", "futil_directory"]) / "fud" / "bitstream" / "xrt.ini"
        )
        sim_script = (
            Path(config["global", "futil_directory"])
            / "fud"
            / "bitstream"
            / "sim_script.tcl"
        )
        mode = config["stages", self.name, "mode"]

        # remote execution
        remote_exec = RemoteExecution(self, config)

        @builder.step()
        def check_host_cpp():
            """
            Make sure that `-s wdb.host` is provided
            """
            if host_cpp is None:
                raise errors.MissingDynamicConfiguration("wdb.host")

        @builder.step()
        def compile_host(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Compile the host code
            """
            cmd = (
                f"cd {tmpdir} && "
                "g++ "
                f"-I{xrt_location}/include "
                f"-I{xilinx_location}/include "
                "-Wall -O0 -g -std=c++14 -fmessage-length=0 "
                "host.cpp "
                "-o 'host' "
                f"-L{xrt_location}/lib -lOpenCL -lpthread -lrt -lstdc++"
            )
            self._shell(client, cmd)

        @builder.step()
        def generate_emconfig(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Generate emconfig.json
            """
            cmd = (
                f"cd {tmpdir} && "
                f"{xilinx_location}/bin/emconfigutil "
                f"--platform {self.device} "
                "--od ."
            )
            self._shell(client, cmd)

        @builder.step()
        def emulate(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Emulation the xclbin
            """
            cmd = (
                f"cd {tmpdir} && {setup_commands} && "
                f"XCL_EMULATION_MODE={mode} "
                f"./host kernel.xclbin {self.device}"
            )
            self._shell(client, cmd)

        # Schedule
        check_host_cpp()
        input_data = builder.input()

        file_map = {
            input_data: "kernel.xclbin",
            host_cpp: "host.cpp",
            xrt: "xrt.ini",
            sim_script: "sim_script.tcl",
        }
        if remote_exec.use_ssh:
            remote_exec.import_libs()
            client, tmpdir = remote_exec.open_and_send(file_map)
        else:
            sandbox = LocalSandbox(self, save_temps)
            tmpdir = sandbox.create(file_map)
            client = Source(None, SourceType.UnTyped)

        compile_host(client, tmpdir)
        generate_emconfig(client, tmpdir)
        emulate(client, tmpdir)

        wdb_name = f"{self.device}-0-kernel.wdb"
        if remote_exec.use_ssh:
            return remote_exec.close_and_get(
                client,
                tmpdir,
                wdb_name,
                keep_tmpdir=save_temps,
            )
        else:
            return sandbox.get_file(wdb_name)
