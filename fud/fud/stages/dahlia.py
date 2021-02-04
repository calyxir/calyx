from fud.stages import SourceType, Stage

from ..utils import shell, unwrap_or


class DahliaStage(Stage):
    def __init__(self, config, dest, flags):
        super().__init__("dahlia", dest, SourceType.Path, SourceType.Stream, config)
        self.flags = flags
        self.setup()

    def _define_steps(self, input_data):
        cmd = [
            self.cmd,
            unwrap_or(self.config["stages", self.name, "flags"], ""),
            self.flags,
        ]

        @self.step(description=" ".join(cmd))
        def run_dahlia(dahlia_prog: SourceType.Path) -> SourceType.Stream:
            return shell(cmd + [str(dahlia_prog)])

        return run_dahlia(input_data)
