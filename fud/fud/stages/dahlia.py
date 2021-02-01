from fud.stages import Stage, SourceType
from ..utils import unwrap_or


class DahliaStage(Stage):
    def __init__(self, config):
        super().__init__("dahlia", "futil", SourceType.Path, SourceType.Stream, config)
        self.setup()

    def _define_steps(self, input_data):
        cmd = [
            self.cmd,
            unwrap_or(self.config["stages", self.name, "flags"], ""),
            "-b futil --lower -l error",
        ]

        @self.step(
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description=" ".join(cmd),
        )
        def run_dahlia(step, dahlia_prog):
            return step.shell(cmd + [dahlia_prog])

        return run_dahlia(input_data)
