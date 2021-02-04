from fud.stages import Stage, SourceType
from ..utils import shell


class VcdumpStage(Stage):
    def __init__(self, config):
        super().__init__(
            "vcd",
            "vcd_json",
            SourceType.Stream,
            SourceType.Stream,
            config,
            "Transform VCD file to JSON",
        )
        self.setup()

    def _define_steps(self, stream):
        @self.step(description=f"{self.cmd} --pretty")
        def run_vcdump(inp_stream: SourceType.Stream) -> SourceType.Stream:
            return shell(f"{self.cmd} --pretty", stdin=inp_stream)

        return run_vcdump(stream)
