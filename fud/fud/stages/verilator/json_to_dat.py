import json
import numpy as np
from .numeric_types import FixedPoint, Bitnum
from pathlib import Path
from decimal import Decimal
from fud.errors import InvalidNumericType, Malformed


# Converts `val` into a bit string that is `bw` characters wide
def bitstring(val, bw):
    # first truncate val by `bw` bits
    val &= 2 ** bw - 1
    return "{:x}".format(val)


def parse_dat(path, args):
    """Parses a number with the given numeric type
    arguments from the array at the given `path`.
    """
    if not path.exists():
        raise Malformed(
            "Data directory",
            (
                f"Output file for memory `{path.stem}' is missing. "
                "This probably happened because a memory is specified in the "
                "input JSON file but is not marked with @external(1) in the "
                "Calyx program. Either add the @external(1) in front of the cell "
                "definition for the memory or remove it from the JSON file."
            ),
        )

    NumericType = FixedPoint if "int_width" in args else Bitnum
    with path.open("r") as f:
        return np.array([
            NumericType(f"0x{value}", **args).str_value()
            for value in f.readlines()
        ])


def parse_fp_widths(format):
    """Returns the width and int_width from the given
    format. We need only two of following three in the
    numeric type format:
        (width, int_width, frac_width)
    The third can then be inferred.
    """
    int_width = format.get("int_width")
    frac_width = format.get("frac_width")
    width = format.get("width")

    def provided(x, y):
        # Returns whether x and y are provided,
        # i.e. they are not None.
        return x is not None and y is not None

    if provided(width, int_width):
        return width, int_width
    elif provided(int_width, frac_width):
        return (int_width + frac_width), int_width
    elif provided(width, frac_width):
        return width, (width - frac_width)
    else:
        raise Exception(
            """Fixed point requires one of the following:
            (1) Bit width `width`, integer width `int_width`.
            (2) Bit width `width`, fractional width `frac_width`.
            (3) Integer width `int_width`, fractional width `frac_width`.
            """
        )


def convert2dat(output_dir, data, extension):
    """Goes through the JSON data and creates
    a file for each key, flattens the data,
    and then converts it to bitstrings.
    Also generates a file named "shape.json" that contains information to
    de-parse the memory files.
    Only memory files corresponding to the fields in shape.json should be
    deparsed.
    """
    output_dir = Path(output_dir)
    shape = {}
    for k, item in data.items():
        path = output_dir / f"{k}.{extension}"
        path.touch()
        arr = np.array(item["data"])
        format = item["format"]

        numeric_type = format["numeric_type"]
        is_signed = format["is_signed"]
        shape[k] = {"is_signed": is_signed}

        if numeric_type not in {"bitnum", "fixed_point"}:
            raise InvalidNumericType(numeric_type)

        is_fp = numeric_type == "fixed_point"
        if is_fp:
            width, int_width = parse_fp_widths(format)
            shape[k]["int_width"] = int_width
        else:
            # `Bitnum`s only have a bit width.
            width = format["width"]
        shape[k]["width"] = width

        def convert(x):
            NumericType = FixedPoint if is_fp else Bitnum
            return NumericType(x, **shape[k]).hex_string(with_prefix=False)

        with path.open("w") as f:
            for v in arr.flatten():
                f.write(convert(v) + "\n")

        shape[k]["shape"] = list(arr.shape)
        shape[k]["numeric_type"] = numeric_type

    # Commit shape.json file.
    shape_json_file = output_dir / "shape.json"
    with shape_json_file.open("w") as f:
        json.dump(shape, f, indent=2)


def convert2json(input_dir, extension):
    """Converts a directory of *.dat
    files back into a JSON file.
    Only de-parses output memory files corresponding to memory names in
    "shape.json"
    """
    input_dir = Path(input_dir)
    shape_json_path = input_dir / "shape.json"
    if not shape_json_path.exists():
        return {}

    data = {}
    shape_json = json.load(shape_json_path.open("r"))

    for (mem, form) in shape_json.items():
        path = input_dir / f"{mem}.{extension}"
        args = form.copy()
        args.pop('shape')
        args.pop('numeric_type')
        arr = parse_dat(path, args)
        if form["shape"] == [0]:
            raise Malformed(
                "Data format shape",
                (
                    f"Memory '{mem}' has shape 0. "
                    "This happens if the `data` field is set to `[]`. "
                    "If you want the memory printed out in the output JSON, "
                    "remove its definition from the input JSON file. "
                    "If you want it to be printed in the output JSON, "
                    f"set the data field of '{mem}' to all zeros with the "
                    "correct dimensions."
                ),
            )

        try:
            arr = arr.reshape(tuple(form["shape"]))
        except Exception:
            raise Malformed("Data format shape", f"Memory '{mem}' had invalid shape.")

        data[mem] = arr.tolist()

    return data
