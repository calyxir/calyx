from tempfile import TemporaryDirectory

from .. import errors
from fud.stages import Source, SourceType, Stage, Step


class VivadoTemplateStage(Stage):
    """
    TODO(rachit): Document this.
    """

    def __init__(self, name, target, config, descr):
        super().__init__(name, target, config, descr)
        self.use_ssh = False
        self.ssh_host = None
        self.ssh_user = None
        self.ssh_client = None
        self.scp_client = None

    def _config_ssh(self):
        if self.config["stages", self.name, "remote"] is not None:
            # dynamically import libraries if they are installed
            try:
                from paramiko import SSHClient as ssh
                from scp import SCPClient as scp

                self.ssh_client = ssh
                self.scp_client = scp
            except ModuleNotFoundError:
                raise errors.RemoteLibsNotInstalled

            self.use_ssh = True
            self.ssh_host = self.config["stages", self.name, "ssh_host"]
            self.ssh_user = self.config["stages", self.name, "ssh_username"]
        else:
            self.use_ssh = False

    def _establish_connection(self, steps):
        # maybe establish ssh connection
        if self.use_ssh:
            ssh_connection = Step(SourceType.Passthrough)

            def f(inp, ctx):
                if self.use_ssh:
                    ssh = self.ssh_client()
                    ssh.load_system_host_keys()
                    ssh.connect(self.ssh_host, username=self.ssh_user)
                    ctx["ssh_client"] = ssh
                return (inp, None, 0)

            ssh_connection.set_func(f, "Connect to server over SSH")
            steps.append(ssh_connection)

    def _mktmp(self, steps):
        # make temporary directory
        mktmp = Step(SourceType.Passthrough)

        def f(inp, ctx):
            if self.use_ssh:
                _, stdout, _ = ctx["ssh_client"].exec_command("mktemp -d")
                tmpdir = stdout.read().decode("ascii").strip()
                ctx["tmpdir"] = tmpdir
            else:
                tmpdir = TemporaryDirectory()
                ctx["tmpdir"] = tmpdir.name
                ctx["tmpdir_obj"] = tmpdir
            return (inp, None, 0)

        mktmp.set_func(f, "Make temporary directory.")
        steps.append(mktmp)

    def _move_files(self, steps, device_files, src_file):
        # copy over files
        move = Step(SourceType.Path)
        if self.use_ssh:

            def f(inp, ctx):
                with self.scp_client(ctx["ssh_client"].get_transport()) as scp:
                    scp.put(device_files, remote_path=ctx["tmpdir"])
                    scp.put(inp.data, remote_path=f'{ctx["tmpdir"]}/{src_file}')
                return (inp, None, 0)

            move.set_func(f, "Copy synth files over SCP.")
        else:
            move.set_cmd(
                " ".join(
                    [
                        "cp",
                        " ".join(device_files),
                        "{ctx[tmpdir]}",
                        "&&",
                        f"cp {{ctx[input_path]}} {{ctx[tmpdir]}}/{src_file}",
                    ]
                )
            )
        steps.append(move)

    def _finalize_ssh(self, steps):
        if self.use_ssh:
            copy = Step(SourceType.Passthrough)

            def f(inp, ctx):
                if self.use_ssh:
                    tmpdir = TemporaryDirectory()
                    with self.scp_client(ctx["ssh_client"].get_transport()) as scp:
                        scp.get(
                            ctx["tmpdir"], local_path=f"{tmpdir.name}", recursive=True
                        )
                        ctx["old_tmpdir"] = ctx["tmpdir"]
                        ctx["tmpdir"] = tmpdir.name
                        ctx["tmpdir_obj"] = tmpdir
                    return (inp, None, 0)

            copy.set_func(f, "Copy files back.")
            steps.append(copy)

            close_ssh = Step(SourceType.Passthrough)

            def f(inp, ctx):
                if self.use_ssh:
                    ctx["ssh_client"].exec_command(f'rm -r {ctx["tmpdir"]}')
                    ctx["ssh_client"].close()
                return (inp, None, 0)

            close_ssh.set_func(f, "Close SSH")
            steps.append(close_ssh)

            restructure_tmp = Step(SourceType.Passthrough)
            restructure_tmp.set_cmd(
                " ".join(
                    [
                        "mv {ctx[tmpdir]}/tmp.*/* {ctx[tmpdir]}",
                        "&&",
                        "rm -r {ctx[tmpdir]}/tmp.*",
                    ]
                )
            )
            steps.append(restructure_tmp)

    def _output_dir(self, steps):
        # output directory
        output = Step(SourceType.Passthrough)

        def f(_, ctx):
            return (Source(ctx["tmpdir_obj"], SourceType.TmpDir), None, 0)

        output.set_func(f, "Output synthesis directory.")
        steps.append(output)
