from fud.stages import Stage, SourceType
from ..utils import unwrap_or


class FutilStage(Stage):
    def __init__(self, config, destination, flags, desc):
        self.flags = flags
        super().__init__(
            "futil", destination, SourceType.Stream, SourceType.Stream, config, desc
        )
        self.setup()

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
        def run_futil(step, inp_stream: SourceType.Stream) -> SourceType.Stream:
            return step.shell(
                cmd,
                stdin=inp_stream,
            )

        return run_futil(input_data)
