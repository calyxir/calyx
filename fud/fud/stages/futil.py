from fud.stages import SourceType, Stage
from fud import config as cfg

from ..utils import shell, unwrap_or


class CalyxStage(Stage):
    name = "calyx"

    def __init__(self, destination, flags, desc):
        self.flags = flags
        super().__init__(
            src_state="calyx",
            target_state=destination,
            input_type=SourceType.Stream,
            output_type=SourceType.Stream,
            description=desc,
        )

    @staticmethod
    def defaults():
        return {}

    def known_opts(self):
        return ["flags", "exec", "file_extensions"]

    def _define_steps(self, input, builder, config):
        calyx_exec = config["stages", self.name, "exec"]
        cmd = " ".join(
            [
                calyx_exec,
                "-l",
                config["global", cfg.ROOT],
                self.flags,
                unwrap_or(config["stages", self.name, "flags"], ""),
            ]
        )

        @builder.step(description=cmd)
        def run_futil(inp_stream: SourceType.Stream) -> SourceType.Stream:
            return shell(
                cmd,
                stdin=inp_stream,
            )

        return run_futil(input)
