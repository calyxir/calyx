from fud.stages import Stage, SourceType, Source
from pathlib import Path
import simplejson as sjson
import numpy as np
from fud.stages.verilator.numeric_types import FixedPoint, Bitnum
from fud.errors import InvalidNumericType, Malformed
from fud.stages.verilator.json_to_dat import parse_fp_widths
from calyx.utils import float_to_fixed_point


from ..utils import shell, unwrap_or, TmpDir

_FILE_NAME = "data.json"


class InterpreterStage(Stage):
    def __init__(self, config, flags, desc):
        super().__init__(
            "interpreter",
            "interpreter-out",
            SourceType.Stream,
            SourceType.Stream,
            config,
            desc,
        )

        self.flags = flags
        self.data_path = self.config["stages", self.name, "data"]

        self.setup()

    def _define_steps(self, input_data):

        cmd = " ".join(
            [
                self.cmd,
                self.flags,
                "-l",
                self.config["global", "futil_directory"],
                "--data {data_file}" if self.data_path else "",
                "{target}",
            ]
        )

        @self.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        @self.step()
        def convert_json_to_interp_json(
            tmpdir: SourceType.Directory, json_path: SourceType.Stream
        ):
            """
            Creates a data file to initialze the interpreter memories
            """
            round_float_to_fixed = self.config[
                "stages", self.name, "round_float_to_fixed"
            ]
            convert_to_json(
                tmpdir.name,
                sjson.load(json_path, use_decimal=True),
                round_float_to_fixed,
            )

        @self.step()
        def interpret(
            target: SourceType.Path, tmpdir: SourceType.Directory
        ) -> SourceType.Stream:
            """
            Invoke the interpreter
            """
            return shell(
                cmd.format(data_file=Path(tmpdir.name) / _FILE_NAME, target=str(target))
            )

        @self.step()
        def cleanup(tmpdir: SourceType.Directory):
            """
            Remove the temporary directory
            """
            tmpdir.remove()

        # schedule

        tmpdir = mktmp()

        if self.data_path is not None:
            convert_json_to_interp_json(
                tmpdir, Source(Path(self.data_path), SourceType.Path)
            )

        result = interpret(input_data, tmpdir)
        cleanup(tmpdir)

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
            with_prefix = False
            if not is_fp:
                return Bitnum(x, **shape[k]).bit_string(with_prefix)

            try:
                return FixedPoint(x, **shape[k]).bit_string(with_prefix)
            except InvalidNumericType as error:
                if round_float_to_fixed:
                    # Only round if it is not already representable.
                    fractional_width = width - int_width
                    x = float_to_fixed_point(float(x), fractional_width)
                    x = str(x)
                    return FixedPoint(x, **shape[k]).bit_string(with_prefix)
                else:
                    raise error

        output_json[k] = list([convert(x) for x in arr.flatten()])
    out_path = output_dir / _FILE_NAME

    with out_path.open("w") as f:
        sjson.dump(output_json, f, indent=2, use_decimal=True)
