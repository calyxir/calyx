import json
import numpy as np
from .fixed_point import fp_to_decimal, decimal_to_fp
from pathlib import Path
from fud.errors import InvalidNumericType, Malformed


# Converts `val` into a bitstring that is `bw` characters wide
def bitstring(val, bw):
    # first truncate val by `bw` bits
    val &= 2 ** bw - 1
    return "{:x}".format(val)


def parse_dat_bitnum(path, bw, is_signed):
    """Parses bitnum numbers of bit width `bw`
    from the array at the given `path`.
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

    def to_decimal(hex_v: str) -> int:
        # Takes in a value in string
        # hexadecimal form, and
        # returns the corresponding
        # integer value.
        v = int(hex_v.strip(), 16)
        if is_signed and v > (2 ** (bw - 1)):
            return -1 * ((2 ** bw) - v)

        return v

    with path.open("r") as f:
        return np.array(list(map(to_decimal, f.readlines())))


def parse_dat_fp(path, width, int_width, is_signed):
    """Parses fixed point numbers in the array
     at `path` with the following form:
    Total width: `width`
    Integer width: `int_width`
    Fractional width: `width` - `int_width`
    """

    def hex_to_decimal(v):
        # Given a fixed point number in hexadecimal form,
        # returns the string form of the decimal value,
        # since Decimal is not JSON serializable.
        decimal_v = fp_to_decimal(
                np.binary_repr(int(v.strip(), 16), width),
                width,
                int_width,
                is_signed
            )
        return str(decimal_v).strip('\"')

    with path.open("r") as f:
        return np.array(list(map(hex_to_decimal, f.readlines())))


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

        # Every numeric format shares these two fields.
        numeric_type = format["numeric_type"]
        is_signed = format["is_signed"]
        shape[k] = {
            "shape": list(arr.shape),
            "numeric_type": numeric_type,
            "is_signed": is_signed,
        }

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

        convert = (
            lambda x: decimal_to_fp(x, width, int_width, is_signed) if is_fp else x
        )

        with path.open("w") as f:
            for v in arr.flatten():
                f.write(bitstring(convert(v), width) + "\n")

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
        numeric_type = form["numeric_type"]
        is_signed = form["is_signed"]
        width = form["width"]

        if numeric_type == "bitnum":
            arr = parse_dat_bitnum(path, width, is_signed)
        elif numeric_type == "fixed_point":
            arr = parse_dat_fp(path, width, form["int_width"], is_signed)
        else:
            raise InvalidNumericType(numeric_type)

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
