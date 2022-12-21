from fud.stages import SourceType, Stage
from fud.utils import shell, unwrap_or


class DahliaStage(Stage):
    name = "dahlia"

    def __init__(self, dest, flags, descr):
        super().__init__(
            src_state="dahlia",
            target_state=dest,
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description=descr,
        )
        self.flags = flags

    def known_opts(self):
        return ["flags", "exec", "file_extensions"]

    def _define_steps(self, input, builder, config):
        dahlia_exec = config["stages", self.name, "exec"]
        cmd = " ".join(
            [
                dahlia_exec,
                unwrap_or(config["stages", self.name, "flags"], ""),
                self.flags,
                "{prog}",
            ]
        )

        @builder.step(description=cmd)
        def run_dahlia(dahlia_prog: SourceType.Path) -> SourceType.Stream:
            return shell(cmd.format(prog=str(dahlia_prog)))

        return run_dahlia(input)
