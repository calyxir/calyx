import logging
from io import BytesIO
from pathlib import Path

from fud.stages import Source, SourceType, Stage, Step

from ..vivado.extract import hls_extract
from ..vivado.stage_template import VivadoTemplateStage

SSHClient = None
SCPClient = None


class VivadoHLSStage(VivadoTemplateStage):
    def __init__(self, config):
        super().__init__(
            "vivado-hls", "hls-files", config, "Runs HLS synthesis on a Dahlia program"
        )

    def _define(self):
        steps = []

        self._config_ssh()
        self._establish_connection(steps)
        self._mktmp(steps)
        self._move_files(
            steps,
            [
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
            ],
            "kernel.cpp",
        )
        self._run_vivado_hls(steps)
        self._finalize_ssh(steps)
        self._output_dir(steps)

        return steps

    def _run_vivado_hls(self, steps):
        vivado_hls = Step(SourceType.Path)
        if self.use_ssh:

            def f(inp, ctx):
                _, stdout, _ = ctx["ssh_client"].exec_command(
                    f'cd {ctx["tmpdir"]} && vivado_hls -f hls.tcl'
                )
                for chunk in iter(lambda: stdout.readline(2048), ""):
                    logging.debug(chunk.strip())

                return (inp, None, 0)

            ssh_addr = f"{self.ssh_user}@{self.ssh_host}"
            vivado_hls.set_func(
                f, f'ssh {ssh_addr} cd {{ctx["tmpdir"]}} && vivado_hls -f hls.tcl'
            )
        else:
            vivado_hls.set_cmd(
                " ".join(["cd {ctx[tmpdir]}", "&&", "vivado_hls -f hls.tcl >&2"])
            )
        steps.append(vivado_hls)


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
        extract = Step(SourceType.Passthrough)

        def f(inp, _):
            res = None
            if inp.source_type == SourceType.TmpDir:
                res = hls_extract(Path(inp.data.name))
            else:
                res = hls_extract(Path(inp.data))
            return (Source(BytesIO(res.encode("UTF-8")), SourceType.File), None, 0)

        extract.set_func(f, "Extract information.")

        return [extract]
