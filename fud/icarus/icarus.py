import re
import simplejson as sjson
from pathlib import Path

from fud.stages import Stage, SourceType, Source
from fud.utils import shell, TmpDir, log
from fud.stages.verilator.json_to_dat import convert2dat, convert2json
from fud.stages import futil
import fud.errors as errors


class IcarusBaseStage(Stage):
    """
    Stage to run Verilog programs with Icarus Verilog
    """

    name = "icarus-verilog"

    def __init__(self, is_vcd, desc):
        super().__init__(
            src_state="icarus-verilog",
            target_state="vcd" if is_vcd else "dat",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description=desc,
        )
        self.is_vcd = is_vcd
        self.object_name = "main.vvp"

    @staticmethod
    def pre_install():
        pass

    @staticmethod
    def defaults():
        parent = Path(__file__).parent.resolve()
        test_bench = parent / "./tb.sv"
        return {
            "exec": "iverilog",
            "testbench": str(test_bench.resolve()),
            "round_float_to_fixed": True,
        }

    def known_opts(self):
        return ["exec", "testbench", "round_float_to_fixed"]

    def _define_steps(self, input_data, builder, config):
        testbench = config["stages", self.name, "testbench"]
        cmd = config["stages", self.name, "exec"]

        # Step 1: Make a new temporary directory
        @builder.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        # Step 2a: Dynamically retrieve the value of stages.verilog.data
        @builder.step(
            description="Dynamically retrieve the value of stages.verilog.data"
        )
        def get_verilog_data() -> SourceType.Path:
            data_path = config.get(["stages", "verilog", "data"])
            path = Path(data_path) if data_path else None
            return Source(path, SourceType.Path)

        # Step 2b: check if we need verilog.data to be passes
        @builder.step()
        def check_verilog_for_mem_read(
            verilog_src: SourceType.String, data_path: SourceType.Path
        ):
            """
            Read input verilog to see if `icarus-verilog.data` needs to be set.
            """
            # If verilog.data exists, do nothing
            if not data_path.data and "readmemh" in verilog_src:
                raise errors.MissingDynamicConfiguration("verilog.data")

        # Step 2: Transform data from JSON to Dat.
        @builder.step()
        def json_to_dat(tmp_dir: SourceType.Directory, json_path: SourceType.Path):
            """
            Converts a `json` data format into a series of `.dat` files.
            """
            round_float_to_fixed = config["stages", self.name, "round_float_to_fixed"]
            # if verilog.data was not given, do nothing
            if json_path.data:
                convert2dat(
                    tmp_dir.name,
                    sjson.load(open(json_path.data), use_decimal=True),
                    "dat",
                    round_float_to_fixed,
                )

        # Step 3: compile with verilator
        cmd = " ".join(
            [
                cmd,
                "-g2012",
                "-o",
                "{exec_path}",
                testbench,
                "{input_path}",
            ]
        )

        @builder.step(description=cmd)
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
        @builder.step()
        def simulate(tmpdir: SourceType.Directory) -> SourceType.Stream:
            """
            Simulates compiled icarus verilog program.
            """
            cycle_limit = config["stages", "verilog", "cycle_limit"]
            return shell(
                [
                    f"{tmpdir.name}/{self.object_name}",
                    f"+DATA={tmpdir.name}",
                    f"+CYCLE_LIMIT={str(cycle_limit)}",
                    f"+OUT={tmpdir.name}/output.vcd",
                    f"+NOTRACE={0 if self.is_vcd else 1}",
                ]
            )

        # Step 5(self.vcd == True): extract
        @builder.step()
        def output_vcd(tmpdir: SourceType.Directory) -> SourceType.Stream:
            """
            Return the generated `output.vcd`.
            """
            # return stream instead of path because tmpdir gets deleted
            # before the next stage runs
            return (Path(tmpdir.name) / "output.vcd").open("rb")

        # Step 5(self.vcd == False): extract cycles + data
        @builder.step()
        def output_json(
            simulated_output: SourceType.String, tmpdir: SourceType.Directory
        ) -> SourceType.Stream:
            """
            Convert .dat files back into a json file
            """
            found = re.search(r"reached limit of\s+(\d+) cycles", simulated_output)
            if found is not None:
                raise errors.CycleLimitedReached("verilog", found.group(1))

            r = re.search(r"Simulated\s+((-)?\d+) cycles", simulated_output)
            cycle_count = int(r.group(1)) if r is not None else 0
            if cycle_count < 0:
                log.warn("Cycle count is less than 0")
            data = {
                "cycles": cycle_count,
                "memories": convert2json(tmpdir.name, "out"),
            }

            # Write to a file so we can return a stream.
            out = Path(tmpdir.name) / "output.json"
            with out.open("w") as f:
                sjson.dump(data, f, indent=2, sort_keys=True, use_decimal=True)
            return out.open("rb")

        @builder.step()
        def cleanup(tmpdir: SourceType.Directory):
            """
            Cleanup build files
            """
            tmpdir.remove()

        # Schedule
        tmpdir = mktmp()
        data_path = get_verilog_data()
        # data_path_exists: bool = (
        #     config.get(["stages", "verilog", "data"]) or
        #     config.get(["stages", "mrxl", "data"])
        # )

        # if we need to, convert dynamically sourced json to dat
        check_verilog_for_mem_read(input_data, data_path)
        # otherwise, convert
        json_to_dat(tmpdir, data_path)

        compile_with_iverilog(input_data, tmpdir)
        stdout = simulate(tmpdir)
        result = None
        if self.is_vcd:
            result = output_vcd(tmpdir)
        else:
            result = output_json(stdout, tmpdir)
        cleanup(tmpdir)
        return result


class FutilToIcarus(futil.CalyxStage):
    """
    Stage to transform Calyx into icarus-verilog simulatable Verilog
    """

    # No name since CalyxStage already defines names

    @staticmethod
    def pre_install():
        pass

    def __init__(self):
        super().__init__(
            "icarus-verilog",
            "-b verilog --disable-verify",
            "Compile Calyx to Verilog instrumented for simulation",
        )


class IcarusToVCDStage(IcarusBaseStage):
    """
    Stage to generate VCD files by simulating through Icarus
    """

    def __init__(self):
        super().__init__(True, "Runs Verilog programs with Icarus and generates VCD")


class IcarusToJsonStage(IcarusBaseStage):
    """
    Stage to generate VCD files by simulating through Icarus
    """

    def __init__(self):
        super().__init__(
            False,
            "Runs Verilog programs with Icarus and generates JSON memory file",
        )


# Export the defined stages to fud
__STAGES__ = [FutilToIcarus, IcarusToVCDStage, IcarusToJsonStage]
