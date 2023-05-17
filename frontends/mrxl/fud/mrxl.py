from fud.stages import Stage, SourceType, Source
from fud.utils import shell
from pathlib import Path

from fud.errors import MissingDynamicConfiguration


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
    def pre_install():
        pass

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


class MrXLDataStage(Stage):
    """
    Stage that invokes the MrXL data converter.
    """

    name = "mrxl-data"

    def __init__(self):
        """
        Initialize this stage. Initializing a stage *does not* construct its
        computation graph.
        """
        super().__init__(
            src_state="mrxl-data",
            target_state="verilog-data",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description="Compiles MrXL-native input to Calyx-native input.",
        )

    @staticmethod
    def pre_install():
        pass

    @staticmethod
    def defaults():
        """
        Specify defaults that should be added to fud's configuration file when
        this stage is registered.
        """
        return {"exec": "mrxl-data"}

    def _define_steps(self, input, builder, config):
        """
        Define the steps that will execute in this stage. Each step represents
        a delayed computation that will occur when the stage is executed.
        """

        # Commands at the top-level are evaluated when the computation is being
        # staged
        cmd = config["stages", "mrxl", "exec"]
        mrxl_prog = config.get(["stages", "mrxl", "prog"])

        # Computations within a step are delayed from being executed until
        # the full execution pipeline is generated.
        @builder.step()
        def convert_mrxl_data_to_calyx_data(
            data_path: SourceType.Path, mrxl_prog: SourceType.Path
        ) -> SourceType.Stream:
            """
            Converts MrXL input into calyx input
            """
            return shell(f"{cmd} {str(mrxl_prog)} --data {data_path} --convert")

        # Define a schedule using the steps.
        # A schedule *looks* like an imperative program but actually represents
        # a computation graph that is executed later on.
        if mrxl_prog is None:
            raise MissingDynamicConfiguration("mrxl.prog")
        return convert_mrxl_data_to_calyx_data(
            input, Source(Path(mrxl_prog), SourceType.Path)
        )


# Export the defined stages to fud
__STAGES__ = [MrXLStage, MrXLDataStage]
