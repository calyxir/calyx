import simplejson as sjson
import re
from pathlib import Path

from fud import errors
from fud.stages import Source, SourceType, Stage
from fud.utils import TmpDir, shell
from fud import config as cfg

from .json_to_dat import convert2dat, convert2json

VCD_FILE = "output.vcd"


class JsonToDat(Stage):
    name = "to-dat"

    def __init__(self):
        super().__init__(
            src_state="mem-json",
            target_state="mem-dat",
            input_type=SourceType.Stream,
            output_type=SourceType.Directory,
            description="Converts JSON data to Dat.",
        )

    def _define_steps(self, input: Source, builder, config):
        @builder.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        @builder.step()
        def json_to_dat(json: SourceType.Stream, dir: SourceType.Directory):
            """
            Converts a `json` data format into a series of `.dat` files inside the given
            temporary directory.
            """
            round_float_to_fixed = bool(
                config.get(["stages", self.name, "round_float_to_fixed"])
            )
            convert2dat(
                dir.name,
                sjson.load(json, use_decimal=True),
                "dat",
                round_float_to_fixed,
            )

        dir = mktmp()
        json_to_dat(input, dir)
        return dir


class DatToJson(Stage):
    name = "to-json"

    def __init__(self):
        super().__init__(
            src_state="mem-dat",
            target_state="mem-json",
            input_type=SourceType.Directory,
            output_type=SourceType.Stream,
            description="Converts JSON data to Dat.",
        )

    def _define_steps(self, input: Source, builder, config):
        extension = config["stages", self.name, "extension"]

        @builder.step()
        def output_json(input: SourceType.Directory) -> SourceType.Stream:
            """
            Convert .dat files back into a JSON file
            """
            data = convert2json(input.name, extension)

            tmp = TmpDir()
            # Write to a file so we can return a stream.
            out = Path(tmp.name) / "output.json"
            with out.open("w") as f:
                sjson.dump(data, f, indent=2, sort_keys=True, use_decimal=True)
            return out.open("rb")

        return output_json(input)


class VerilatorStage(Stage):
    name = "verilog"

    def __init__(self, mem, desc):
        super().__init__(
            src_state="verilog",
            target_state=mem,
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description=desc,
        )

        if mem not in ["vcd", "dat"]:
            raise Exception("mem has to be 'vcd' or 'dat'")
        self.vcd = mem == "vcd"

    def known_opts(self):
        return [
            "data",
            "exec",
            "round_float_to_fixed",
            "cycle_limit",
            "file_extensions",
        ]

    def _define_steps(self, input_data, builder, config):
        # Step 1: Make a new temporary directory
        @builder.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        # Step 2a: Dynamically retrieve the value of stages.verilog.data
        @builder.step(description="Dynamically retrieve the value of stages.verilog.data")
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
            Read input verilog to see if `verilog.data` needs to be set.
            """
            # If verilog.data exists, do nothing
            if not data_path.data and "readmemh" in verilog_src:
                raise errors.MissingDynamicConfiguration("verilog.data")

        # Step 2: Transform data from JSON to Dat.
        @builder.step()
        def json_to_dat(tmp_dir: SourceType.Directory, json_path: SourceType.Path):
            """
            Converts a `json` data format into a series of `.dat` files inside the given
            temporary directory.
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
        testbench_sv = str(
            Path(config["global", cfg.ROOT]) / "fud" / "icarus" / "tb.sv"
        )
        cmd = " ".join(
            [
                config["stages", self.name, "exec"],
                "--trace",
                "{input_path}",
                testbench_sv,
                "--binary",
                "--top-module",
                "TOP",  # The wrapper module name from `tb.sv`.
                "--Mdir",
                "{tmpdir_name}",
                "-fno-inline",
            ]
        )

        @builder.step(description=cmd)
        def compile_with_verilator(
            input_path: SourceType.Path, tmpdir: SourceType.Directory
        ) -> SourceType.Stream:
            return shell(
                cmd.format(input_path=str(input_path), tmpdir_name=tmpdir.name),
                stdout_as_debug=True,
            )

        # Step 4: simulate
        @builder.step()
        def simulate(tmpdir: SourceType.Directory) -> SourceType.Stream:
            """
            Simulates compiled Verilator code.
            """
            cycle_limit = config["stages", self.name, "cycle_limit"]
            return shell(
                [
                    f"{tmpdir.name}/VTOP",
                    f"+DATA={tmpdir.name}",
                    f"+CYCLE_LIMIT={str(cycle_limit)}",
                    f"+OUT={tmpdir.name}/output.vcd",
                    f"+NOTRACE={0 if self.vcd else 1}",
                ]
            )

        # Step 5(self.vcd == True): extract
        @builder.step()
        def output_vcd(tmpdir: SourceType.Directory) -> SourceType.Stream:
            """
            Return the generated `output.vcd`.
            """
            # return stream instead of path because tmpdir gets deleted before
            # the next stage runs
            target = Path(tmpdir.name) / VCD_FILE
            return target.open("rb")

        # Step 5(self.vcd == False): extract cycles + data
        @builder.step()
        def output_json(
            simulated_output: SourceType.String, tmpdir: SourceType.Directory
        ) -> SourceType.Stream:
            """
            Convert .dat files back into a json and extract simulated cycles from log.
            """
            # Verify we haven't hit the cycle limit.
            found = re.search(r"reached limit of (\d+) cycles", simulated_output)
            if found is not None:
                raise errors.CycleLimitedReached(self.name, found.group(1))

            # Look for output like: "Simulated 91 cycles"
            r = re.search(r"Simulated\s+((-)?\d+) cycles", simulated_output)
            data = {
                "cycles": int(r.group(1)) if r is not None else 0,
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
            Cleanup Verilator build files that we no longer need.
            """
            tmpdir.remove()

        # Schedule
        tmpdir = mktmp()
        data_path = get_verilog_data()

        # if we need to, convert dynamically sourced json to dat
        check_verilog_for_mem_read(input_data, data_path)
        json_to_dat(tmpdir, data_path)

        compile_with_verilator(input_data, tmpdir)
        stdout = simulate(tmpdir)
        result = output_vcd(tmpdir) if self.vcd else output_json(stdout, tmpdir)
        cleanup(tmpdir)
        return result
