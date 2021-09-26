from fud.stages import Stage, SourceType
from fud.utils import shell


class MrXLStage(Stage):
    """
    Stage that invokes the MrXL frontend.
    """

    def __init__(self, config):
        super().__init__(
            name="mrxl",
            target_stage="futil",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            config=config,
            description="Compiles MrXL to Calyx.",
        )
        self.setup()

    @staticmethod
    def defaults():
        return {
            "exec": "mrxl"
        }

    def _define_steps(self, input_path):
        @self.step(description=self.cmd)
        def run_mrxl(mrxl_prog: SourceType.Path) -> SourceType.Stream:
            return shell(f"{self.cmd} {str(mrxl_prog)}")

        return run_mrxl(input_path)


# Export the defined stages to fud
__STAGES__ = [
    MrXLStage
]
