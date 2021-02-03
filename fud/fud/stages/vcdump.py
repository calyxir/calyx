from fud.stages import Stage, SourceType


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
        def run_vcdump(step, inp_stream: SourceType.Stream) -> SourceType.Stream:
            return step.shell(f"{self.cmd} --pretty", stdin=inp_stream)

        return run_vcdump(stream)
