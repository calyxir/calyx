from fud.stages import Stage, SourceType
from fud.utils import shell


class VcdumpStage(Stage):
    name = "vcd"

    def __init__(self):
        super().__init__(
            src_state="vcd",
            target_state="vcd_json",
            input_type=SourceType.Stream,
            output_type=SourceType.Stream,
            description="Transform VCD file to JSON using `vcdump`",
        )

    def _define_steps(self, stream, builder, config):
        cmd = " ".join([config["stages", self.name, "exec"], "--pretty"])

        @builder.step(description=cmd)
        def run_vcdump(inp_stream: SourceType.Stream) -> SourceType.Stream:
            return shell(cmd, stdin=inp_stream)

        return run_vcdump(stream)
