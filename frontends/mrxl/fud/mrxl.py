from fud.stages import Stage, SourceType, Source
from fud.utils import shell, TmpDir
from fud.errors import MissingDynamicConfiguration

from pathlib import Path

# The temporary filename used for converting mrxl.data to verilog.data
_DATA_FILE = "data.json"


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
            target_state="calyx",
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
        return {"exec": "mrxl", "flags": ""}

    def _define_steps(self, input, builder, config):
        """
        Define the steps that will execute in this stage. Each step represents
        a delayed computation that will occur when the stage is executed.
        """

        # Commands at the top-level are evaluated when the computation is being
        # staged
        cmd = config["stages", self.name, "exec"]
        flags = config.get(("stages", self.name, "flags")) or ""

        # Computations within a step are delayed from being executed until
        # the full execution pipeline is generated.
        @builder.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        @builder.step(description="Set stages.mrxl.prog as `input`")
        def set_mrxl_prog(mrxl_prog: SourceType.Path):
            config["stages", "mrxl", "prog"] = str(mrxl_prog)

        @builder.step(
            description="Save verilog.data in `tmpdir` and update stages.verilog.data"
        )
        def save_data(tmpdir: SourceType.Directory, verilog_data: SourceType.String):
            save_loc = Path(tmpdir.name) / _DATA_FILE

            with open(save_loc, "w") as out:
                out.write(verilog_data)

            config["stages", "verilog", "data"] = save_loc

        @builder.step(description=cmd)
        def run_mrxl(mrxl_prog: SourceType.Path) -> SourceType.Stream:
            return shell(f"{cmd} {str(mrxl_prog)} {flags}")

        # Define a schedule using the steps.
        # A schedule *looks* like an imperative program but actually represents
        # a computation graph that is executed later on.
        mrxl_data = config.get(["stages", "mrxl", "data"])

        if mrxl_data is not None:
            tmpdir = mktmp()

            set_mrxl_prog(input)
            mrxl_data_stage = MrXLDataStage()
            mrxl_data_stage_input = Source.path(mrxl_data)

            builder.ctx.append("mrxl-data")
            verilog_data = builder.also_do(
                mrxl_data_stage_input, mrxl_data_stage, config
            )
            builder.ctx.pop()
            verilog_data = builder.convert_source_to(verilog_data, SourceType.String)

            save_data(tmpdir, verilog_data)
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
        return {}

    def _define_steps(self, input, builder, config):
        """
        Define the steps that will execute in this stage. Each step represents
        a delayed computation that will occur when the stage is executed.
        """

        # Commands at the top-level are evaluated when the computation is being
        # staged
        cmd = config["stages", "mrxl", "exec"]

        # Computations within a step are delayed from being executed until
        # the full execution pipeline is generated.
        @builder.step(description="Dynamically retrieve the value of stages.mrxl.prog")
        def get_mrxl_prog() -> SourceType.Path:
            return Source(Path(config.get(["stages", "mrxl", "prog"])), SourceType.Path)

        @builder.step()
        def convert_mrxl_data_to_calyx_data(
            data_path: SourceType.Path, mrxl_prog: SourceType.Path
        ) -> SourceType.Stream:
            """
            Converts MrXL input into calyx input
            """
            return shell(f"{cmd} {str(mrxl_prog.data)} --data {data_path} --convert")

        # Define a schedule using the steps.
        # A schedule *looks* like an imperative program but actually represents
        # a computation graph that is executed later on.

        mrxl_prog = get_mrxl_prog()

        if mrxl_prog is None:
            raise MissingDynamicConfiguration("mrxl.prog")
        return convert_mrxl_data_to_calyx_data(input, mrxl_prog)


# Export the defined stages to fud
__STAGES__ = [MrXLStage, MrXLDataStage]
