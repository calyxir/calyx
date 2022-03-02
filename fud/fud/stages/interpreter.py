import base64
from fud.stages import Stage, SourceType, Source
from pathlib import Path
import simplejson as sjson
import numpy as np
from fud.stages.verilator.numeric_types import FixedPoint, Bitnum
from fud.errors import InvalidNumericType
from fud.stages.verilator.json_to_dat import parse_fp_widths, float_to_fixed
from fud.utils import shell, TmpDir, unwrap_or, transparent_shell

# A local constant used only within this file largely for organizational
# purposes and to avoid magic strings
_FILE_NAME = "data.json"
_DEBUGGER_TARGET = "debugger"


class InterpreterStage(Stage):

    name = "interpreter"

    @classmethod
    def debugger(cls, interp_flags, debug_flags, desc):
        self = cls(
            interp_flags,
            debug_flags,
            desc,
            output_name=_DEBUGGER_TARGET,
            output_type=SourceType.Terminal,
        )
        self._no_spinner = True
        return self

    def __init__(
        self,
        flags,
        debugger_flags,
        desc,
        output_type=SourceType.Stream,
        output_name="interpreter-out",
    ):
        super().__init__(
            src_state="interpreter",
            target_state=output_name,
            input_type=SourceType.Stream,
            output_type=output_type,
            description=desc,
        )

        self.flags = flags
        self.debugger_flags = debugger_flags

    def _is_debugger(self):
        """
        Am I a debugger?
        """
        return self.target_state == _DEBUGGER_TARGET

    def _define_steps(self, input_data, builder, config):

        script = config["stages", self.name, "exec"]
        data_path = config["stages", "verilog", "data"]

        cmd = [
            script,
            self.flags,
            unwrap_or(config["stages", self.name, "flags"], ""),
            "-l",
            config["global", "futil_directory"],
            "--data" if data_path else "",
            "{data_file}" if data_path else "",
            "{target}",
        ]

        if self._is_debugger():
            cmd += [
                "debug",
                self.debugger_flags,
                unwrap_or(config["stages", self.name, "debugger", "flags"], ""),
            ]

        cmd = " ".join(cmd)

        @builder.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        @builder.step()
        def convert_json_to_interp_json(
            tmpdir: SourceType.Directory, json_path: SourceType.Stream
        ):
            """
            Creates a data file to initialze the interpreter memories
            """
            round_float_to_fixed = config["stages", self.name, "round_float_to_fixed"]
            convert_to_json(
                tmpdir.name,
                sjson.load(json_path, use_decimal=True),
                round_float_to_fixed,
            )

        @builder.step(description=cmd)
        def interpret(
            target: SourceType.Path, tmpdir: SourceType.Directory
        ) -> SourceType.Stream:
            """
            Invoke the interpreter
            """

            command = cmd.format(
                data_file=Path(tmpdir.name) / _FILE_NAME, target=str(target)
            )

            return shell(command)

        @builder.step(description=cmd)
        def debug(
            target: SourceType.Path, tmpdir: SourceType.Directory
        ) -> SourceType.Terminal:
            """
            Invoke the debugger
            """
            command = cmd.format(
                data_file=Path(tmpdir.name) / _FILE_NAME, target=str(target)
            )
            transparent_shell(command)

        @builder.step()
        def parse_output(
            output: SourceType.Stream, json_path: SourceType.UnTyped
        ) -> SourceType.String:
            """
            Parses a raw interpreter output
            """

            return sjson.dumps(parse_from_json(output, json_path))

        # schedule
        tmpdir = mktmp()

        if data_path is not None:
            convert_json_to_interp_json(
                tmpdir, Source(Path(data_path), SourceType.Path)
            )

        if self._is_debugger():
            debug(input_data, tmpdir)
        else:
            result = interpret(input_data, tmpdir)

            if "--raw" in cmd:
                return parse_output(result, Source(data_path, SourceType.UnTyped))
            else:
                return result


def convert_to_json(output_dir, data, round_float_to_fixed):
    output_dir = Path(output_dir)
    shape = {}
    output_json = {}
    for k, item in data.items():
        arr = np.array(item["data"], str)
        format = item["format"]

        numeric_type = format["numeric_type"]
        is_signed = format["is_signed"]
        shape[k] = {"is_signed": is_signed}

        if numeric_type not in {"bitnum", "fixed_point"}:
            raise InvalidNumericType('Fud only supports "fixed_point" and "bitnum".')

        is_fp = numeric_type == "fixed_point"
        if is_fp:
            width, int_width = parse_fp_widths(format)
            shape[k]["int_width"] = int_width
        else:
            width = format["width"]

        shape[k]["width"] = width

        def convert(x):

            if not is_fp:
                return Bitnum(x, **shape[k]).base_64_encode()

            try:
                return FixedPoint(x, **shape[k]).base_64_encode()
            except InvalidNumericType as error:
                if round_float_to_fixed:
                    # Only round if it is not already representable.
                    fractional_width = width - int_width
                    x = float_to_fixed(float(x), fractional_width)
                    x = str(x)
                    return FixedPoint(x, **shape[k]).base_64_encode()
                else:
                    raise error

        output_json[k] = [convert(x) for x in arr.flatten()]
    out_path = output_dir / _FILE_NAME

    with out_path.open("w") as f:
        sjson.dump(output_json, f, indent=2, use_decimal=True)


def parse_from_json(output_data_str, original_data_file_path):
    if original_data_file_path is not None:
        with Path(original_data_file_path).open("r") as f:
            orig = sjson.load(f)
    else:
        orig = None

    output_data = sjson.load(output_data_str)

    output_data = output_data["memories"]

    def parse_entry(target, format_details):
        numeric_type, is_signed, width = (
            format_details
            if format_details is not None
            else (
                "bitnum",
                False,
                None,
            )
        )

        if isinstance(target, list):
            return [parse_entry(x, (numeric_type, is_signed, width)) for x in target]
        elif isinstance(target, str):
            num = base64.standard_b64decode(target)
            if numeric_type == "bitnum":
                return int.from_bytes(num, "little", signed=is_signed)
            else:
                assert False, f"got {numeric_type}"

    processed_output_data = dict()

    for component, inner_dict in output_data.items():
        inner_dict_output = dict()
        for key, target in inner_dict.items():
            if orig is not None:
                format_details = (
                    orig[key]["format"]["numeric_type"],
                    orig[key]["format"]["is_signed"],
                    orig[key]["format"]["width"],
                )
            else:
                format_details = None

            inner_dict_output[key] = parse_entry(target, format_details)
        processed_output_data[component] = inner_dict_output

    return processed_output_data
