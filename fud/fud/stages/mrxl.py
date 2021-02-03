from fud.stages import Stage, SourceType


class MrXLStage(Stage):
    """
    Stage that invokes the MrXL frontend.
    """

    def __init__(self, config):
        super().__init__(
            "mrxl",
            "futil",
            SourceType.Path,
            SourceType.Stream,
            config,
            "Compiles MrXL to FuTIL.",
        )
        self.setup()

    def _define_steps(self, input_path):
        @self.step(description=self.cmd)
        def run_mrxl(step, mrxl_prog: SourceType.Path) -> SourceType.Stream:
            return step.shell(f"{self.cmd} {str(mrxl_prog)}")

        return run_mrxl(input_path)
