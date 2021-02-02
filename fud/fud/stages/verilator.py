import json
import re
from pathlib import Path

from fud.stages import Source, SourceType, Stage

from .. import errors
from ..json_to_dat import convert2dat, convert2json
from ..utils import TmpDir


class VerilatorStage(Stage):
    def __init__(self, config, mem, desc):
        super().__init__(
            "verilog", mem, SourceType.Path, SourceType.Stream, config, desc
        )

        if mem not in ["vcd", "dat"]:
            raise Exception("mem has to be 'vcd' or 'dat'")
        self.vcd = mem == "vcd"
        self.testbench_files = [
            str(
                Path(self.config["global", "futil_directory"])
                / "fud"
                / "sim"
                / "testbench.cpp"
            ),
            str(
                Path(self.config["global", "futil_directory"])
                / "fud"
                / "sim"
                / "wrapper.cpp"
            ),
        ]
        self.data_path = self.config["stages", self.name, "data"]
        self.setup()

    def _define_steps(self, input_data):
        # Step 1: Make a new temporary directory
        @self.step(input_type=SourceType.Null, output_type=SourceType.Directory)
        def mktmp(step):
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        # Step 2a: check if we need verilog.data to be passes
        @self.step(input_type=SourceType.String, output_type=SourceType.Null)
        def check_verilog_for_mem_read(step, verilog_src):
            """
            Read input verilog to see if `verilog.data` needs to be set.
            """
            if "readmemh" in verilog_src:
                raise errors.MissingDynamicConfiguration("verilog.data")

        # Step 2: Transform data from JSON to Dat.
        @self.step(input_type=(SourceType.Directory, SourceType.Stream))
        def json_to_dat(step, tmp_dir, json_path):
            """
            Converts a `json` data format into a series of `.dat` files.
            """
            convert2dat(tmp_dir.name, json.load(json_path), "dat")

        # Step 3: compile with verilator
        cmd = " ".join(
            [
                self.cmd,
                "-cc",
                "--trace",
                "{input_path}",
                "--exe " + " --exe ".join(self.testbench_files),
                "--build",
                "--top-module",
                self.config["stages", self.name, "top_module"],
                "--Mdir",
                "{tmpdir_name}",
            ]
        )

        @self.step(
            input_type=(SourceType.Path, SourceType.Directory),
            output_type=SourceType.Stream,
            description=cmd,
        )
        def compile_with_verilator(step, input_path, tmpdir):
            return step.shell(
                cmd.format(input_path=input_path, tmpdir_name=tmpdir.name),
                stdout_as_debug=True,
            )

        # Step 4: simulate
        @self.step(
            input_type=SourceType.Directory,
            output_type=SourceType.Stream,
        )
        def simulate(step, tmpdir):
            """
            Simulates compiled Verilator code.
            """
            return step.shell(
                [
                    f"DATA={tmpdir.name}",
                    f"{tmpdir.name}/Vmain",
                    f"{tmpdir.name}/output.vcd",
                    str(self.config["stages", self.name, "cycle_limit"]),
                    # Don't trace if we're only looking at memory outputs
                    "--trace" if self.vcd else "",
                ]
            )

        # Step 5(self.vcd == True): extract
        @self.step(input_type=SourceType.Directory, output_type=SourceType.Stream)
        def output_vcd(step, tmpdir):
            """
            Return the generated `output.vcd`.
            """
            return (Path(tmpdir.name) / "output.vcd").open("rb")

        # Step 5(self.vc == False): extract cycles + data
        @self.step(
            input_type=(SourceType.String, SourceType.Directory),
            output_type=SourceType.String,
        )
        def output_json(step, simulated_output, tmpdir):
            """
            Convert .dat files back into a json and extract simulated cycles from log.
            """
            # Simulated 91 cycles
            r = re.search(r"Simulated (\d+) cycles", simulated_output)
            data = {
                "cycles": int(r.group(1)),
                "memories": convert2json(tmpdir.name, "out"),
            }
            return json.dumps(data, indent=2, sort_keys=True)

        @self.step(input_type=SourceType.Directory)
        def cleanup(step, tmpdir):
            """
            Cleanup Verilator build files that we no longer need.
            """
            tmpdir.remove()

        # Schedule
        tmpdir = mktmp()
        # if we need to, convert dynamically sourced json to dat
        if self.data_path is None:
            check_verilog_for_mem_read(input_data)
        else:
            json_to_dat(tmpdir, Source(self.data_path, SourceType.Path))
        compile_with_verilator(input_data, tmpdir)
        stdout = simulate(tmpdir)
        result = None
        if self.vcd:
            result = output_vcd(tmpdir)
        else:
            result = output_json(stdout, tmpdir)
        cleanup(tmpdir)
        return result
