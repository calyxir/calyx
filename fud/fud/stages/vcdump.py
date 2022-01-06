from fud.stages import Stage, SourceType
from fud.utils import shell


class VcdumpStage(Stage):
    name = "vcd"

    def __init__(self, config):
        super().__init__(
            src_state="vcd",
            target_state="vcd_json",
            input_type=SourceType.Stream,
            output_type=SourceType.Stream,
            config=config,
            description="Transform VCD file to JSON",
        )
        self.setup()

    def _define_steps(self, stream):
        @self.step(description=f"{self.cmd} --pretty")
        def run_vcdump(inp_stream: SourceType.Stream) -> SourceType.Stream:
            return shell(f"{self.cmd} --pretty", stdin=inp_stream)

        return run_vcdump(stream)
