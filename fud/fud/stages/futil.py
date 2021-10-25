from fud.stages import SourceType, Stage

from ..utils import shell, unwrap_or


class FutilStage(Stage):
    def __init__(self, config, destination, flags, desc):
        self.flags = flags
        super().__init__(
            "futil", destination, SourceType.Stream, SourceType.Stream, config, desc
        )
        self.setup()

    @staticmethod
    def defaults():
        return {}

    def _define_steps(self, input_data):
        cmd = " ".join(
            [
                self.cmd,
                "-l",
                self.config["global", "futil_directory"],
                self.flags,
                unwrap_or(self.config["stages", self.name, "flags"], ""),
            ]
        )

        @self.step(description=cmd)
        def run_futil(inp_stream: SourceType.Stream) -> SourceType.Stream:
            return shell(
                cmd,
                stdin=inp_stream,
            )

        return run_futil(input_data)
