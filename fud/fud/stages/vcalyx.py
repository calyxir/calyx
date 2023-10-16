from fud.stages import Stage, SourceType, Source
from pathlib import Path
import simplejson as sjson
import numpy as np
from fud.stages.verilator.numeric_types import FixedPoint, Bitnum
from fud.errors import InvalidNumericType
from fud.stages.verilator.json_to_dat import parse_fp_widths, float_to_fixed
from fud.utils import shell, TmpDir, unwrap_or, transparent_shell
from fud import config as cfg
from enum import Enum, auto

# A local constant used only within this file largely for organizational
# purposes and to avoid magic strings
_FILE_NAME = "data.json"

class VCalyxStage(Stage):
    name = "vcalyx"

    def __init__(
        self,
        flags,
        debugger_flags,
        desc,
        output_type=SourceType.Stream,
        output_name="vcalyx-out",
    ):
        super().__init__(
            src_state="calyx-sexp",
            target_state=output_name,
            input_type=SourceType.Stream,
            output_type=output_type,
            description=desc,
        )

        self.flags = flags
        self.debugger_flags = debugger_flags

    def _define_steps(self, input_data, builder, config):
        script = config["stages", self.name, "exec"]
        data_path_exists: bool = (config["stages", "verilog", "data"] or
                                  config.get(["stages", "mrxl", "data"]))

        cmd = [
            script,
            self.flags,
            unwrap_or(config["stages", self.name, "flags"], ""),
            "-l",
            config["global", cfg.ROOT],
            "--data" if data_path_exists else "",
            "{data_file}" if data_path_exists else "",
            "{target}",
        ]

        cmd = " ".join(cmd)

        @builder.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        @builder.step(description="Dynamically retrieve the value of stages.verilog.data")
        def get_verilog_data() -> SourceType.Path:
            data_path = config.get(["stages", "verilog", "data"])
            return Path(data_path) if data_path else None

        @builder.step()
        def output_data(
            tmpdir: SourceType.Directory,
        ) -> SourceType.Path:
            """
            Output converted data for the interpreter-data target
            """
            path = Path(tmpdir.name) / _FILE_NAME
            return path

        @builder.step(description=cmd)
        def interpret(
            target: SourceType.Path, data: SourceType.Path
        ) -> SourceType.Stream:
            """
            Invoke the interpreter
            """

            command = cmd.format(
                data_file=data, target=target
            )

            return shell(command)

        # schedule
        tmpdir = mktmp()
        data_path = get_verilog_data()

        return interpret(input_data, data_path)
