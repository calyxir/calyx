import logging as log
from pathlib import Path
from tempfile import NamedTemporaryFile

from .. import errors
from ..stages import Source, SourceType
from ..utils import shell


class RemoteExecution:
    """
    TODO(rachit): Document this.
    """

    def __init__(self, stage):
        self.stage = stage
        if self.stage.config["stages", self.stage.name, "remote"] is not None:
            self.use_ssh = True
            self.ssh_host = self.stage.config["stages", self.stage.name, "ssh_host"]
            self.ssh_user = self.stage.config["stages", self.stage.name, "ssh_username"]
        else:
            self.use_ssh = False

    def import_libs(self):
        @self.stage.step()
        def import_libs():
            """Import libraries"""
            if self.use_ssh:
                # dynamically import libraries if they are installed
                try:
                    from paramiko import SSHClient
                    from scp import SCPClient

                    self.SSHClient = SSHClient
                    self.SCPClient = SCPClient
                except ModuleNotFoundError:
                    raise errors.RemoteLibsNotInstalled

        import_libs()

    def _open(self):
        """Establish an SSH connection.

        Return a client object and the temporary directory created on the
        remote host.
        """
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

        client = establish_connection()
        tmpdir = mktmp(client)
        return client, tmpdir

    def open_and_send(self, input_files):
        """Connect to the SSH server and send input files.

        `input_files` is a dict that maps local paths to remote paths,
        the latter of which will be relative to the remote temporary
        directory. Each source should be a Path, and each destination
        should be a string (either may be Source-wrapped).

        Return a client object and the temporary directory for the files.
        """

        @self.stage.step()
        def send_file(
            client: SourceType.UnTyped,
            tmpdir: SourceType.String,
            src_path: SourceType.Path,
            dest_path: SourceType.String,
        ):
            """Copy one input file over the SSH channel.
            """
            with self.SCPClient(client.get_transport()) as scp:
                scp.put(
                    src_path,
                    str(Path(tmpdir) / dest_path),
                )

        client, tmpdir = self._open()
        for src_path, dest_path in input_files.items():
            if not isinstance(src_path, Source):
                src_path = Source(src_path, SourceType.Path)
            if not isinstance(dest_path, Source):
                dest_path = Source(dest_path, SourceType.String)
            send_file(client, tmpdir, src_path, dest_path)
        return client, tmpdir

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

    def _close(self, client, remote_tmpdir):
        """Close the SSH connection to the server.

        Also removes the remote temporary directory.
        """
        @self.stage.step()
        def finalize_ssh(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Remove created temporary files and close ssh connection.
            """
            client.exec_command(f"rm -r {tmpdir}")
            client.close()

        finalize_ssh(client, remote_tmpdir)

    def close_and_transfer(self, client, remote_tmpdir, local_tmpdir):
        """Close the SSH connection and fetch the remote files.

        Copy the entire contents of `remote_tmpdir` to `local_tmpdir`.
        """
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

        copy_back(client, remote_tmpdir, local_tmpdir)
        self._close(client, remote_tmpdir)

    def close_and_get(self, client, remote_tmpdir, path):
        """Close the SSH connection and retrieve a single file.

        Produces the resulting downloaded file.
        """
        @self.stage.step()
        def fetch_file(
            client: SourceType.UnTyped,
            remote_tmpdir: SourceType.String,
        ) -> SourceType.Path:
            """Download a file over SSH.
            """
            src_path = Path(remote_tmpdir) / path
            with NamedTemporaryFile("wb", delete=False) as tmpfile:
                dest_path = tmpfile.name
            with self.SCPClient(client.get_transport()) as scp:
                scp.get(src_path, dest_path)
            return dest_path.open("rb")

        local_path = fetch_file(client, remote_tmpdir)
        self._close(client, remote_tmpdir)
        return local_path
