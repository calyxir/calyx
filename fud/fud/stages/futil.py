from fud.stages import Stage, Step, SourceType
from ..utils import unwrap_or


class FutilStage(Stage):
    def __init__(self, config, destination, flags, desc):
        self.flags = flags
        super().__init__("futil", destination, config, desc)

    def _define(self):
        main = Step(SourceType.File)
        main.set_cmd(
            " ".join(
                [
                    self.cmd,
                    "-l",
                    self.config["global", "futil_directory"],
                    self.flags,
                    unwrap_or(self.config["stages", self.name, "flags"], ""),
                ]
            )
        )
        return [main]
