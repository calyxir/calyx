from io import BytesIO
from pathlib import Path
from tempfile import TemporaryDirectory

import logging

from fud.stages import Source, SourceType, Stage, Step

from ..vivado.extract import hls_extract
from .. import errors

SSHClient = None
SCPClient = None


class VivadoHLSStage(Stage):
    def __init__(self, config):
        super().__init__(
            "vivado-hls", "hls-files", config, "Runs HLS synthesis on a Dahlia program"
        )

    def _define(self):
        if self.config['stages', self.name, 'remote'] is not None:
            # dynamically import libraries if they are installed
            try:
                from paramiko import SSHClient as ssh
                from scp import SCPClient as scp
                global SSHClient
                global SCPClient
                SSHClient = ssh
                SCPClient = scp
            except ModuleNotFoundError:
                raise errors.RemoteLibsNotInstalled

            self.use_ssh = True
            self.ssh_host = self.config['stages', self.name, 'ssh_host']
            self.ssh_user = self.config['stages', self.name, 'ssh_username']
        else:
            self.use_ssh = False

        steps = []

        self._establish_connection(steps)
        self._mktmp(steps)
        self._move_files(steps)
        self._run_vivado_hls(steps)
        self._finalize_ssh(steps)

        # output directory
        output = Step(SourceType.Nothing)

        def f(inp, ctx):
            return (Source(ctx["tmpdir_obj"], SourceType.TmpDir), None, 0)

        output.set_func(f, "Output synthesis directory.")
        steps.append(output)

        return steps

    def _establish_connection(self, steps):
        # maybe establish ssh connection
        if self.use_ssh:
            ssh_connection = Step(SourceType.Nothing)

            def f(inp, ctx):
                if self.use_ssh:
                    ssh = SSHClient()
                    ssh.load_system_host_keys()
                    ssh.connect(self.ssh_host, username=self.ssh_user)
                    ctx['ssh_client'] = ssh
                return (inp, None, 0)

            ssh_connection.set_func(f, 'Connect to server over SSH')
            steps.append(ssh_connection)

    def _mktmp(self, steps):
        # make temporary directory
        mktmp = Step(SourceType.Nothing)

        def f(inp, ctx):
            if self.use_ssh:
                _, stdout, _ = ctx['ssh_client'].exec_command('mktemp -d')
                tmpdir = stdout.read().decode('ascii').strip()
                ctx["tmpdir"] = tmpdir
            else:
                tmpdir = TemporaryDirectory()
                ctx["tmpdir"] = tmpdir.name
                ctx["tmpdir_obj"] = tmpdir
            return (inp, None, 0)

        mktmp.set_func(f, "Make temporary directory.")
        steps.append(mktmp)

    def _move_files(self, steps):
        # copy over files
        move = Step(SourceType.Path)
        synth_files = [
            str(
                Path(self.config["global", "futil_directory"])
                / "fud"
                / "synth"
                / "hls.tcl"
            ),
            str(
                Path(self.config["global", "futil_directory"])
                / "fud"
                / "synth"
                / "fxp_sqrt.h"
            ),
        ]
        if self.use_ssh:
            def f(inp, ctx):
                with SCPClient(ctx['ssh_client'].get_transport()) as scp:
                    scp.put(synth_files, remote_path=ctx['tmpdir'])
                    scp.put(inp.data, remote_path=f'{ctx["tmpdir"]}/kernel.cpp')
                return (inp, None, 0)
            move.set_func(f, "Copy synth files over SCP.")
        else:
            move.set_cmd(
                " ".join(
                    [
                        "cp",
                        " ".join(synth_files),
                        "{ctx[tmpdir]}",
                        "&&",
                        "cp {ctx[input_path]} {ctx[tmpdir]}/kernel.cpp",
                    ]
                )
            )
        steps.append(move)

    def _run_vivado_hls(self, steps):
        vivado_hls = Step(SourceType.Path)
        if self.use_ssh:
            def f(inp, ctx):
                _, stdout, _ = ctx['ssh_client'].exec_command(f'cd {ctx["tmpdir"]} && vivado_hls -f hls.tcl')
                for chunk in iter(lambda: stdout.readline(2048), ""):
                    logging.debug(chunk)

                return (inp, None, 0)
            ssh_addr = f'{self.ssh_user}@{self.ssh_host}'
            vivado_hls.set_func(f, f'ssh {ssh_addr} cd {{ctx["tmpdir"]}} && vivado_hls -f hls.tcl')
        else:
            vivado_hls.set_cmd(
                " ".join(["cd {ctx[tmpdir]}", "&&", "vivado_hls -f hls.tcl >&2"])
            )
        steps.append(vivado_hls)

    def _finalize_ssh(self, steps):
        if self.use_ssh:
            copy = Step(SourceType.Nothing)

            def f(inp, ctx):
                if self.use_ssh:
                    tmpdir = TemporaryDirectory()
                    with SCPClient(ctx['ssh_client'].get_transport()) as scp:
                        scp.get(ctx["tmpdir"], local_path=f"{tmpdir.name}", recursive=True)
                        ctx["old_tmpdir"] = ctx["tmpdir"]
                        ctx["tmpdir"] = tmpdir.name
                        ctx["tmpdir_obj"] = tmpdir
                    return (inp, None, 0)

            copy.set_func(f, "Copy files back.")
            steps.append(copy)

            close_ssh = Step(SourceType.Nothing)

            def f(inp, ctx):
                if self.use_ssh:
                    ctx['ssh_client'].exec_command(f'rm -r {ctx["tmpdir"]}')
                    ctx['ssh_client'].close()
                return (inp, None, 0)

            close_ssh.set_func(f, 'Close SSH')
            steps.append(close_ssh)

            restructure_tmp = Step(SourceType.Nothing)
            restructure_tmp.set_cmd(' '.join([
                'mv {ctx[tmpdir]}/tmp.*/* {ctx[tmpdir]}',
                '&&',
                'rmdir {ctx[tmpdir]}/tmp.*'
            ]))
            steps.append(restructure_tmp)


class VivadoHLSExtractStage(Stage):
    def __init__(self, config):
        super().__init__(
            "hls-files",
            "hls-estimate",
            config,
            "Runs HLS synthesis on a Dahlia program",
        )

    def _define(self):
        # make temporary directory
        extract = Step(SourceType.Nothing)

        def f(inp, ctx):
            res = None
            if inp.source_type == SourceType.TmpDir:
                res = hls_extract(Path(inp.data.name))
            else:
                res = hls_extract(Path(inp.data))
            return (Source(BytesIO(res.encode("UTF-8")), SourceType.File), None, 0)

        extract.set_func(f, "Extract information.")

        return [extract]
