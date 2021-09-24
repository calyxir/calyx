import simplejson as sjson
from pathlib import Path

from fud.stages import Stage, SourceType, Source
from fud.utils import shell, TmpDir
from fud.stages.verilator.json_to_dat import convert2dat, convert2json
import fud.errors as errors


class IcarusBaseStage(Stage):
    """
    Stage to run Verilog programs with Icarus Verilog
    """

    def __init__(self, is_vcd, desc, config):
        super().__init__(
            name="icarus-verilog",
            target_stage="vcd" if is_vcd else "dat",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            config=config,
            description=desc,
        )
        self.is_vcd = is_vcd
        self.testbench = config["stages", self.name, "testbench"]
        self.runtime = config["stages", self.name, "runtime"]
        try:
            self.data_path = config["stages", "verilog", "data"]
        except errors.UnsetConfiguration:
            self.data_path = None
        self.object_name = "main.vvp"
        self.setup()

    @staticmethod
    def defaults():
        parent = Path(__file__).parent.resolve()
        test_bench = parent / "./tb.sv"
        return {
            "exec": "iverilog",
            "runtime": "vvp",
            "testbench": str(test_bench.resolve()),
            "round_float_to_fixed": True,
        }

    def _define_steps(self, input_data):
        # Step 1: Make a new temporary directory
        @self.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        # Step 2a: check if we need verilog.data to be passes
        @self.step()
        def check_verilog_for_mem_read(verilog_src: SourceType.String):
            """
            Read input verilog to see if `icarus-verilog.data` needs to be set.
            """
            if "readmemh" in verilog_src:
                raise errors.MissingDynamicConfiguration("verilog.data")

        # Step 2: Transform data from JSON to Dat.
        @self.step()
        def json_to_dat(tmp_dir: SourceType.Directory, json_path: SourceType.Stream):
            """
            Converts a `json` data format into a series of `.dat` files.
            """
            round_float_to_fixed = self.config[
                "stages", self.name, "round_float_to_fixed"
            ]
            convert2dat(
                tmp_dir.name,
                sjson.load(json_path, use_decimal=True),
                "dat",
                round_float_to_fixed,
            )

        # Step 3: compile with verilator
        cmd = " ".join(
            [
                self.cmd,
                "-g2012",
                "-o",
                "{exec_path}",
                self.testbench,
                "{input_path}",
            ]
        )

        @self.step(description=cmd)
        def compile_with_iverilog(
            input_path: SourceType.Path, tmpdir: SourceType.Directory
        ) -> SourceType.Stream:
            return shell(
                cmd.format(
                    input_path=str(input_path),
                    exec_path=f"{tmpdir.name}/{self.object_name}",
                ),
                stdout_as_debug=True,
            )

        # Step 4: simulate
        @self.step()
        def simulate(tmpdir: SourceType.Directory) -> SourceType.Stream:
            """
            Simulates compiled icarus verilog program.
            """
            return shell(
                [
                    f"{tmpdir.name}/{self.object_name}",
                    f"+DATA={tmpdir.name}",
                    f"+OUT={tmpdir.name}/output.vcd",
                ]
            )

        # Step 5(self.vcd == True): extract
        @self.step()
        def output_vcd(tmpdir: SourceType.Directory) -> SourceType.Stream:
            """
            Return the generated `output.vcd`.
            """
            # return stream instead of path because tmpdir gets deleted
            # before the next stage runs
            return (Path(tmpdir.name) / "output.vcd").open("rb")

        # Step 5(self.vc == False): extract cycles + data
        @self.step()
        def output_json(
            simulated_output: SourceType.String, tmpdir: SourceType.Directory
        ) -> SourceType.String:
            """
            Convert .dat files back into a json file
            """
            data = {
                "memories": convert2json(tmpdir.name, "out"),
            }
            return sjson.dumps(data, indent=2, sort_keys=True, use_decimal=True)

        @self.step()
        def cleanup(tmpdir: SourceType.Directory):
            """
            Cleanup build files
            """
            tmpdir.remove()

        # Schedule
        tmpdir = mktmp()
        # if we need to, convert dynamically sourced json to dat
        if self.data_path is None:
            check_verilog_for_mem_read(input_data)
        else:
            json_to_dat(tmpdir, Source(Path(self.data_path), SourceType.Path))
        compile_with_iverilog(input_data, tmpdir)
        stdout = simulate(tmpdir)
        result = None
        if self.is_vcd:
            result = output_vcd(tmpdir)
        else:
            result = output_json(stdout, tmpdir)
        cleanup(tmpdir)
        return result


class IcarusToVCDStage(IcarusBaseStage):
    """
    Stage to generate VCD files by simulating through Icarus
    """

    def __init__(self, config):
        super().__init__(
            True, "Runs Verilog programs with Icarus and generates VCD", config
        )


class IcarusToJsonStage(IcarusBaseStage):
    """
    Stage to generate VCD files by simulating through Icarus
    """

    def __init__(self, config):
        super().__init__(
            False,
            "Runs Verilog programs with Icarus and generates JSON memory file",
            config,
        )


# Export the defined stages to fud
__STAGES__ = [IcarusToVCDStage, IcarusToJsonStage]
