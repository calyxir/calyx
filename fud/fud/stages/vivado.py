import logging
from io import BytesIO
from pathlib import Path

from fud.stages import Source, SourceType, Stage, Step

from ..vivado.extract import futil_extract
from ..vivado.stage_template import VivadoTemplateStage

SSHClient = None
SCPClient = None


class VivadoStage(VivadoTemplateStage):
    def __init__(self, config):
        super().__init__(
            "synth-verilog",
            "synth-files",
            config,
            "Runs synthesis on a Verilog program",
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
                    / "synth.tcl"
                ),
                str(
                    Path(self.config["global", "futil_directory"])
                    / "fud"
                    / "synth"
                    / "device.xdc"
                ),
            ],
            "main.sv",
        )
        self._run_vivado(steps)
        self._finalize_ssh(steps)
        self._output_dir(steps)

        return steps

    def _run_vivado(self, steps):
        vivado = Step(SourceType.Path)
        if self.use_ssh:

            def f(inp, ctx):
                _, stdout, _ = ctx["ssh_client"].exec_command(
                    " ".join(
                        [
                            f"cd {ctx['tmpdir']}",
                            "&&",
                            "vivado -mode batch -source synth.tcl",
                        ]
                    )
                )
                for chunk in iter(lambda: stdout.readline(2048), ""):
                    logging.debug(chunk.strip())

                return (inp, None, 0)

            ssh_addr = f"{self.ssh_user}@{self.ssh_host}"
            vivado.set_func(
                f,
                " ".join(
                    [
                        f"ssh {ssh_addr} 'cd {{ctx[tmpdir]}}",
                        "&&",
                        "vivado -mode batch -source synth.tcl'",
                    ]
                ),
            )
        else:
            vivado.set_cmd(
                " ".join(
                    [
                        "cd {ctx[tmpdir]}",
                        "&&",
                        "vivado -mode batch -source synth.tcl >&2",
                    ]
                )
            )
        steps.append(vivado)


class VivadoExtractStage(Stage):
    def __init__(self, config):
        super().__init__(
            "synth-files",
            "resource-estimate",
            config,
            "Runs synthesis on a Verilog program",
        )

    def _define(self):
        # extract
        extract = Step(SourceType.Passthrough)

        def f(inp, _):
            res = None
            if inp.source_type == SourceType.TmpDir:
                res = futil_extract(Path(inp.data.name))
            else:
                res = futil_extract(Path(inp.data))
            return (Source(BytesIO(res.encode("UTF-8")), SourceType.File), None, 0)

        extract.set_func(f, "Extract information.")

        return [extract]
