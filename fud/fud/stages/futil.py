from fud.stages import SourceType, Stage

from ..utils import shell, unwrap_or


class FutilStage(Stage):
    name = "futil"

    def __init__(self, config, destination, flags, desc):
        self.flags = flags
        super().__init__(
            src_state="futil",
            target_state=destination,
            input_type=SourceType.Stream,
            output_type=SourceType.Stream,
            config=config,
            description=desc,
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
