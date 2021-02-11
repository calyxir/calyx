import logging as log

from .. import errors
from ..stages import SourceType
from ..utils import shell


class RemoteExecution:
    """
    TODO(rachit): Document this.
    """

    def __init__(self, stage):
        self.stage = stage
        self.device_files = self.stage.device_files
        self.target_name = self.stage.target_name
        if self.stage.config["stages", self.stage.name, "remote"] is not None:
            # dynamically import libraries if they are installed
            try:
                from paramiko import SSHClient
                from scp import SCPClient

                self.SSHClient = SSHClient
                self.SCPClient = SCPClient
            except ModuleNotFoundError:
                raise errors.RemoteLibsNotInstalled

            self.use_ssh = True
            self.ssh_host = self.stage.config["stages", self.stage.name, "ssh_host"]
            self.ssh_user = self.stage.config["stages", self.stage.name, "ssh_username"]
        else:
            self.use_ssh = False

    def open_and_transfer(self, input_path):
        @self.stage.step()
        def establish_connection() -> SourceType.UnTyped:
            """
            Establish ssh connection.
            """
            client = self.SSHClient()
            client.load_system_host_keys()
            client.connect(self.ssh_host, username=self.ssh_user)
            return client

        @self.stage.step()
        def mktmp(client: SourceType.UnTyped) -> SourceType.String:
            """
            Execute `mktemp -d` over ssh connection.
            """
            _, stdout, _ = client.exec_command("mktemp -d")
            tmpdir = stdout.read().decode("ascii").strip()
            return tmpdir

        @self.stage.step()
        def send_files(
            client: SourceType.UnTyped,
            verilog_path: SourceType.Path,
            tmpdir: SourceType.String,
        ):
            """
            Copy device files over ssh channel.
            """
            with self.SCPClient(client.get_transport()) as scp:
                scp.put(self.device_files, remote_path=tmpdir)
                scp.put(str(verilog_path), remote_path=f"{tmpdir}/{self.target_name}")

        client = establish_connection()
        tmpdir = mktmp(client)
        send_files(client, input_path, tmpdir)
        return (client, tmpdir)

    def execute(self, client, tmpdir, cmd):
        @self.stage.step()
        def run_vivado(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Run vivado command remotely.
            """
            _, stdout, stderr = client.exec_command(
                " ".join([f"cd {tmpdir}", "&&", cmd])
            )
            # read stdout in 2048 byte chunks so that we get live output streaming in
            # debug mode
            for chunk in iter(lambda: stdout.readline(2048), ""):
                log.debug(chunk.strip())

        run_vivado(client, tmpdir)

    def close_and_transfer(self, client, remote_tmpdir, local_tmpdir):
        @self.stage.step()
        def copy_back(
            client: SourceType.UnTyped,
            remote_tmpdir: SourceType.String,
            local_tmpdir: SourceType.Directory,
        ):
            """
            Copy files generated on server back to local host.
            """
            with self.SCPClient(client.get_transport()) as scp:
                scp.get(
                    remote_tmpdir, local_path=f"{local_tmpdir.name}", recursive=True
                )
                shell(f"mv {local_tmpdir.name}/tmp.* {local_tmpdir.name}")
                shell(f"rm -r {local_tmpdir.name}/tmp.*")

        @self.stage.step()
        def finalize_ssh(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Remove created temporary files and close ssh connection.
            """
            client.exec_command(f"rm -r {tmpdir}")
            client.close()

        copy_back(client, remote_tmpdir, local_tmpdir)
        finalize_ssh(client, remote_tmpdir)
