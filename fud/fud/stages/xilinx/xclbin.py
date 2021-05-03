import logging as log

from pathlib import Path
from paramiko import SSHClient
from scp import SCPClient

from fud.stages import SourceType, Stage, Source
from fud.stages.futil import FutilStage
from fud.utils import TmpDir


class XilinxStage(Stage):
    def __init__(self, config):
        super().__init__(
            "futil",
            "xclbin",
            SourceType.Path,
            SourceType.Stream,
            config,
            "compiles Calyx programs to Xilinx bitstreams",
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

        # remote execution
        self.ssh_host = self.config["stages", self.target_stage, "ssh_host"]
        self.ssh_user = self.config["stages", self.target_stage, "ssh_username"]

        self.mode = self.config["stages", self.target_stage, "mode"]
        self.device = self.config["stages", self.target_stage, "device"]

        self.setup()

    def _define_steps(self, input_data):
        # # Step 1: Make a new temporary directory
        # @self.step()
        # def mktmp() -> SourceType.Directory:
        #     """
        #     Make temporary directory to store generated files.
        #     """
        #     return TmpDir()

        # Step 2: Compile input using `-b xilinx`
        @self.step()
        def compile_xilinx(inp: SourceType.Stream) -> SourceType.Path:
            """
            TODO: write
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
            TODO: write
            """
            return (
                self.xml_futil.run(Source(inp, SourceType.Stream))
                .convert_to(SourceType.Path)
                .data
            )

        @self.step()
        def establish_connection() -> SourceType.UnTyped:
            """
            Establish SSH connection
            """
            client = SSHClient()
            client.load_system_host_keys()
            client.connect(self.ssh_host, username=self.ssh_user)
            return client

        @self.step()
        def make_remote_tmpdir(client: SourceType.UnTyped) -> SourceType.String:
            """
            Execution `mktemp -d` on server.
            """
            _, stdout, _ = client.exec_command("mktemp -d")
            return stdout.read().decode("ascii").strip()

        @self.step()
        def send_files(
            client: SourceType.UnTyped,
            tmpdir: SourceType.String,
            xilinx: SourceType.Path,
            xml: SourceType.Path,
        ):
            """
            Copy files over ssh channel
            """
            with SCPClient(client.get_transport()) as scp:
                scp.put(xilinx, remote_path=f"{tmpdir}/toplevel.v")
                scp.put(xml, remote_path=f"{tmpdir}/kernel.xml")
                scp.put(self.gen_xo_tcl, remote_path=f"{tmpdir}/gen_xo.tcl")

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
            _, stdout, _ = client.exec_command(cmd)

            for chunk in iter(lambda: stdout.readline(2048), ""):
                log.debug(chunk.strip())

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
                    "-lo xclbin/kernel.xclbin",
                    "xclbin/kernel.xo",
                ]
            )
            _, stdout, _ = client.exec_command(cmd)

            for chunk in iter(lambda: stdout.readline(2048), ""):
                log.debug(chunk.strip())

        @self.step()
        def download_xclbin(
            client: SourceType.UnTyped,
            tmpdir: SourceType.String,
        ) -> SourceType.Stream:
            """
            Download xclbin file
            """
            local_tmpdir = TmpDir()
            xclbin_path = Path(local_tmpdir.name) / "kernel.xclbin"
            with SCPClient(client.get_transport()) as scp:
                scp.get(f"{tmpdir}/xclbin/kernel.xclbin", local_path=str(xclbin_path))
            return xclbin_path.open("rb")

        @self.step()
        def cleanup(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Close SSH Connection and cleanup temporaries.
            """
            # client.exec_command("rm -r {tmpdir}")
            print(tmpdir)
            client.close()

        xilinx = compile_xilinx(input_data)
        xml = compile_xml(input_data)
        client = establish_connection()
        tmpdir = make_remote_tmpdir(client)
        send_files(client, tmpdir, xilinx, xml)
        package_xo(client, tmpdir)
        compile_xclbin(client, tmpdir)
        xclbin = download_xclbin(client, tmpdir)
        cleanup(client, tmpdir)
        return xclbin
