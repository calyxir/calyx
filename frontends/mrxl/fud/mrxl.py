from fud.stages import Stage, SourceType
from fud.utils import shell


class MrXLStage(Stage):
    """
    Stage that invokes the MrXL frontend.
    """

    name = "mrxl"

    def __init__(self):
        super().__init__(
            src_state="mrxl",
            target_state="futil",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description="Compiles MrXL to Calyx.",
        )

    @staticmethod
    def defaults():
        return {"exec": "mrxl"}

    def _define_steps(self, input_path, config):
        cmd = config["stages", self.name, "exec"]

        @self.step(description=cmd)
        def run_mrxl(mrxl_prog: SourceType.Path) -> SourceType.Stream:
            return shell(f"{cmd} {str(mrxl_prog)}")

        return run_mrxl(input_path)


# Export the defined stages to fud
__STAGES__ = [MrXLStage]
