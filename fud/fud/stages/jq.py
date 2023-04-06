from fud.stages import Stage, SourceType
from fud.utils import shell


class JqStage(Stage):
    name = "jq"

    def __init__(self, src):
        super().__init__(
            src_state=src,
            target_state="jq",
            input_type=SourceType.Stream,
            output_type=SourceType.Stream,
            description="Run `jq` on a JSON file",
        )

    def _define_steps(self, stream, builder, config):
        file = config.get(["stages", self.name, "file"])
        expr = config.get(["stages", self.name, "expr"])
        flags = config.get(["stages", self.name, "flags"])
        assert not (file and expr), "jq does not support expr and file at the same time"

        cmd = " ".join(
            [
                config["stages", self.name, "exec"],
                "-j",  # don't print newline
                f"-f {file}" if file else "",
                f'"{expr}"' if expr else "",
                f"{flags}" if flags else "",
            ]
        )

        @builder.step(description=cmd)
        def run(inp_stream: SourceType.Stream) -> SourceType.Stream:
            return shell(cmd, stdin=inp_stream)

        return run(stream)
