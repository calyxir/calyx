from fud.stages import Stage, SourceType
from fud.utils import shell


class MrXLStage(Stage):
    """
    Stage that invokes the MrXL frontend.
    """

    name = "mrxl"

    def __init__(self):
        """
        Initialize this stage. Initializing a stage *does not* construct its
        computation graph.
        """
        super().__init__(
            src_state="mrxl",
            target_state="futil",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description="Compiles MrXL to Calyx.",
        )

    @staticmethod
    def defaults():
        """
        Specify defaults that should be added to fud's configuration file when
        this stage is registered.
        """
        return {"exec": "mrxl"}

    def _define_steps(self, input, builder, config):
        """
        Define the steps that will execute in this stage. Each step represents
        a delayed computation that will occur when the stage is executed.
        """

        # Commands at the top-level are evaluated when the computation is being
        # staged
        cmd = config["stages", self.name, "exec"]

        # Computations within a step are delayed from being executed until
        # the full execution pipeline is generated.
        @builder.step(description=cmd)
        def run_mrxl(mrxl_prog: SourceType.Path) -> SourceType.Stream:
            return shell(f"{cmd} {str(mrxl_prog)}")

        # Define a schedule using the steps.
        # A schedule *looks* like an imperative program but actually represents
        # a computation graph that is executed later on.
        return run_mrxl(input)


# Export the defined stages to fud
__STAGES__ = [MrXLStage]
