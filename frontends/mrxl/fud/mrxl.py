from fud.stages import Stage, SourceType, Source
from fud.utils import shell, TmpDir
from pathlib import Path
import json

# Local static variables
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
        data_path = config["stages", "verilog", "data"]

        @builder.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        def convert_mrxl_data_to_calyx_data(
            tmpdir: SourceType.Directory,
            data_path: SourceType.Stream,
            mrxl_prog: SourceType.Path
        ) -> None:
            """
            Converts MrXL input into calyx input
            """
            # banking_factor, bank_size= get_banking_factor(mrxl_prog)
            # data = json.load(data_path)
            # calyx_data = dict()
            # for var, val in data:
            #     val = val["data"]
            #     format = val["format"]
            #     for i in range(banking_factor):
            #         bank = f'var_b{i}'
            #         calyx_data[bank] = {
            #             "data": val[i * bank_size : i * (bank_size + 1)],
            #             "format": format
            #         }
            config.__setitem__(
                ("stages", "verilog", "data"),
                '/scratch/susan/calyx/frontends/mrxl/fud/test_input.data'
            )

        # Computations within a step are delayed from being executed until
        # the full execution pipeline is generated.
        @builder.step(description=cmd)
        def run_mrxl(mrxl_prog: SourceType.Path) -> SourceType.Stream:
            # print("cmd: ", cmd)
            # print("shell command: ", f"{cmd} {str(mrxl_prog)}")
            return shell(f"{cmd} {str(mrxl_prog)}")

        # Define a schedule using the steps.
        # A schedule *looks* like an imperative program but actually represents
        # a computation graph that is executed later on.
        tmpdir = mktmp()

        if data_path is None:
            raise ValueError("verilog.data must be provided")
        # print("data path before convert: ", config["stages", "verilog", "data"])
        # print("data_path: ", data_path)
        convert_mrxl_data_to_calyx_data(
            tmpdir,
            Source(Path(data_path), SourceType.Path),
            input
        )
        # print("data path2: ", config["stages", "verilog", "data"])

        # print("data path: ", config["stages", "verilog", "data"])

        return run_mrxl(input)

# def get_banking_factor(mrxlprog):
# TODO

# Export the defined stages to fud
__STAGES__ = [MrXLStage]
