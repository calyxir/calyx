import simplejson as sjson
import re
from pathlib import Path

from fud import errors
from fud.stages import Source, SourceType, Stage
from fud.utils import TmpDir, shell, unwrap_or

from .json_to_dat import convert2dat, convert2json


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

    def _define_steps(self, builder, config):

        testbench_files = [
            str(
                Path(config["global", "futil_directory"])
                / "fud"
                / "sim"
                / "testbench.cpp"
            ),
        ]
        data_path = config.get(["stages", self.name, "data"])

        # Step 1: Make a new temporary directory
        @builder.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        # Step 2a: check if we need verilog.data to be passes
        @builder.step()
        def check_verilog_for_mem_read(verilog_src: SourceType.String):
            """
            Read input verilog to see if `verilog.data` needs to be set.
            """
            if "readmemh" in verilog_src:
                raise errors.MissingDynamicConfiguration("verilog.data")

        # Step 2: Transform data from JSON to Dat.
        @builder.step()
        def json_to_dat(tmp_dir: SourceType.Directory, json_path: SourceType.Stream):
            """
            Converts a `json` data format into a series of `.dat` files inside the given
            temporary directory.
            """
            round_float_to_fixed = config["stages", self.name, "round_float_to_fixed"]
            convert2dat(
                tmp_dir.name,
                sjson.load(json_path, use_decimal=True),
                "dat",
                round_float_to_fixed,
            )

        # Step 3: compile with verilator
        cmd = " ".join(
            [
                config["stages", self.name, "exec"],
                "-cc",
                "--trace",
                "{input_path}",
                "--exe " + " --exe ".join(testbench_files),
                "--build",
                "--top-module",
                config["stages", self.name, "top_module"],
                "--Mdir",
                "{tmpdir_name}",
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
            return shell(
                [
                    f"{tmpdir.name}/Vmain",
                    unwrap_or(
                        config["stages", self.name, "vcd-target"],
                        f"{tmpdir.name}/output.vcd",
                    ),
                    str(config["stages", self.name, "cycle_limit"]),
                    # Don't trace if we're only looking at memory outputs
                    "--trace" if self.vcd else "",
                    f"+DATA={tmpdir.name}",
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

            if config["stages", self.name, "vcd-target"] is not None:
                target = Path(config["stages", self.name, "vcd-target"])
            else:
                target = Path(tmpdir.name) / "output.vcd"

            return target.open("rb")

        # Step 5(self.vcd == False): extract cycles + data
        @builder.step()
        def output_json(
            simulated_output: SourceType.String, tmpdir: SourceType.Directory
        ) -> SourceType.String:
            """
            Convert .dat files back into a json and extract simulated cycles from log.
            """
            # Verify we haven't hit the cycle limit.
            found = re.search(r"reached limit of (\d+) cycles", simulated_output)
            if found is not None:
                raise errors.CycleLimitedReached(self.name, found.group(1))

            # Look for output like: "Simulated 91 cycles"
            r = re.search(r"Simulated (\d+) cycles", simulated_output)
            data = {
                "cycles": int(r.group(1)) if r is not None else 0,
                "memories": convert2json(tmpdir.name, "out"),
            }
            return sjson.dumps(data, indent=2, sort_keys=True, use_decimal=True)

        @builder.step()
        def cleanup(tmpdir: SourceType.Directory):
            """
            Cleanup Verilator build files that we no longer need.
            """
            tmpdir.remove()

        # Schedule
        input_data = builder.input()
        tmpdir = mktmp()
        # if we need to, convert dynamically sourced json to dat
        if data_path is None:
            check_verilog_for_mem_read(input_data)
        else:
            json_to_dat(tmpdir, Source(Path(data_path), SourceType.Path))
        compile_with_verilator(input_data, tmpdir)
        stdout = simulate(tmpdir)
        result = output_vcd(tmpdir) if self.vcd else output_json(stdout, tmpdir)
        cleanup(tmpdir)
        return result
