import logging as log
from pathlib import Path
from tempfile import NamedTemporaryFile
import shutil

from fud.utils import TmpDir, FreshDir
from .. import errors
from ..stages import Source, SourceType, ComputationGraph, Stage
from ..config import Configuration


class RemoteExecution:
    """A utility for executing commands on a remote SSH server.

    A `RemoteExecution` object gets "attached" to a fud stage. That
    stage can then use the configuration options `remote` (to enable
    remote execution), `ssh_host`, and `ssh_user`. The stage should call
    `open_and_send`, `execute`, and `close_and_*` to create steps to run
    the relevant functionality remotely.
    """

    def __init__(self, builder: ComputationGraph, stage: Stage, config: Configuration):
        self.stage = stage
        self.builder = builder
        self.SSHClient = None
        self.SCPClient = None
        if config["stages", self.stage.name, "remote"] is not None:
            self.use_ssh = True
            self.ssh_host = config["stages", self.stage.name, "ssh_host"]
            self.ssh_user = config["stages", self.stage.name, "ssh_username"]
        else:
            self.use_ssh = False

    def import_libs(self):
        @self.builder.step()
        def import_libs():
            """Import libraries"""
            if self.use_ssh:
                # dynamically import libraries if they are installed
                try:
                    from paramiko import SSHClient
                    from scp import SCPClient  # type: ignore

                    self.SSHClient = SSHClient
                    self.SCPClient = SCPClient
                except ModuleNotFoundError as e:
                    raise errors.RemoteLibsNotInstalled from e

        import_libs()

    def _open(self):
        """Establish an SSH connection.

        Return a client object and the temporary directory created on the
        remote host.
        """

        if self.ssh_host == "" or self.ssh_user == "":
            log.warn(
                "Attempting to use remote execution but SSH host or user look invalid."
                f" Host: `{self.ssh_host}`, user: `{self.ssh_user}`"
            )

        @self.builder.step(
            description=f"Start ssh connection to `{self.ssh_user}@{self.ssh_host}`"
        )
        def establish_connection() -> SourceType.UnTyped:
            """
            Establish ssh connection.
            """
            client = self.SSHClient()
            client.load_system_host_keys()
            client.connect(self.ssh_host, username=self.ssh_user)
            return client

        @self.builder.step()
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

        @self.builder.step()
        def send_file(
            client: SourceType.UnTyped,
            tmpdir: SourceType.String,
            src_path: SourceType.Path,
            dest_path: SourceType.String,
        ):
            """Copy one input file over the SSH channel."""
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
        @self.builder.step(f"SSH execute: {cmd}")
        def run_remote(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """Run a command remotely via SSH."""
            _, stdout, stderr = client.exec_command(
                " ".join([f"cd {tmpdir}", "&&", cmd])
            )
            # read stdout in 2048 byte chunks so that we get live output streaming in
            # debug mode
            for chunk in iter(lambda: stdout.readline(2048), ""):
                log.debug(chunk.strip())

            for chunk in iter(lambda: stderr.readline(2048), ""):
                log.warn(chunk.strip())

            exit_code = stdout.channel.recv_exit_status()
            if exit_code != 0:
                log.error(f"Non-zero exit code: {exit_code}")

        run_remote(client, tmpdir)

    def _close(self, client, remote_tmpdir, keep_tmpdir=False):
        """Close the SSH connection to the server.

        Also removes the remote temporary directory, unless the
        `keep_tmpdir` flag is set.
        """

        @self.builder.step()
        def finalize_ssh(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Remove created temporary files and close ssh connection.
            """
            if not keep_tmpdir:
                client.exec_command(f"rm -r {tmpdir}")
            client.close()

        finalize_ssh(client, remote_tmpdir)

    def close_and_transfer(self, client, remote_tmpdir, local_tmpdir):
        """Close the SSH connection and fetch the remote files.

        Copy the entire contents of `remote_tmpdir` to `local_tmpdir`.
        """

        @self.builder.step()
        def copy_back(
            client: SourceType.UnTyped,
            remote_tmpdir: SourceType.String,
            local_tmpdir: SourceType.Directory,
        ):
            """Copy files generated on server back to local host."""
            with self.SCPClient(client.get_transport()) as scp:
                scp.get(
                    remote_tmpdir, local_path=f"{local_tmpdir.name}", recursive=True
                )

        copy_back(client, remote_tmpdir, local_tmpdir)
        self._close(client, remote_tmpdir)

    def close_and_get(self, client, remote_tmpdir, path, keep_tmpdir=False):
        """Close the SSH connection and retrieve a single file.

        Produces the resulting downloaded file.
        """

        @self.builder.step()
        def fetch_file(
            client: SourceType.UnTyped,
            remote_tmpdir: SourceType.String,
        ) -> SourceType.Path:
            """Download a file over SSH."""
            src_path = Path(remote_tmpdir) / path
            with NamedTemporaryFile("wb", delete=False) as tmpfile:
                dest_path = tmpfile.name
            with self.SCPClient(client.get_transport()) as scp:
                scp.get(src_path, dest_path)
            return Path(dest_path)

        local_path = fetch_file(client, remote_tmpdir)
        self._close(client, remote_tmpdir, keep_tmpdir=keep_tmpdir)
        return local_path


class LocalSandbox:
    """A utility for running commands in a temporary directory.

    This is meant as a local alternative to `RemoteExecution`. Like that
    utility, this provides steps to create a temporary directory,
    execute programs in that temporary directory, and then retrieve
    files from it. However, all this happens locally instead of via SSH.
    """

    def __init__(self, builder, save_temps=False):
        self.builder = builder
        self.save_temps = save_temps

    def create(self, input_files):
        """Copy input files to a fresh temporary directory.

        `input_files` is a dict with the same format as `open_and_send`:
        it maps local Source paths to destination strings.

        Return a path to the newly-created temporary directory.
        """

        def copy_file(tmpdir, src, dst):
            src_str = "{src}" if isinstance(src, Source) else str(src)
            dst_str = "{dst}" if isinstance(dst, Source) else str(dst)
            tmp_str = "{tmpdir}" if isinstance(tmpdir, Source) else str(tmpdir)

            @self.builder.step(description=f"copy {src_str} to {tmp_str}/{dst_str}")
            def copy(
                tmpdir: SourceType.String,
                src_path: SourceType.Path,
                dest_path: SourceType.String,
            ):
                shutil.copyfile(src_path, Path(tmpdir) / dest_path)

            if not isinstance(src, Source):
                src = Source(src, SourceType.Path)
            if not isinstance(dst, Source):
                dst = Source(dst, SourceType.String)

            return copy(tmpdir, src, dst)

        # Schedule
        tmpdir = Source(
            FreshDir() if self.save_temps else TmpDir(),
            SourceType.Directory,
        )

        for src_path, dest_path in input_files.items():
            copy_file(tmpdir, src_path, dest_path)

        self.tmpdir = tmpdir
        return tmpdir

    def get_file(self, name):
        """Retrieve a file from the sandbox directory."""

        @self.builder.step()
        def read_file(
            tmpdir: SourceType.Directory,
            name: SourceType.String,
        ) -> SourceType.Path:
            """Read an output file."""
            return Path(tmpdir.name) / name

        return read_file(
            self.tmpdir,
            Source(name, SourceType.String),
        )
