import json
import numpy as np
from .fixed_point import *
from pathlib import Path


# Converts `val` into a bitstring that is `bw` characters wide
def bitstring(val, bw):
    # first truncate val by `bw` bits
    val &= (2 ** bw - 1)
    return '{:x}'.format(val)


def parse_dat_bitnum(path, bw, is_signed):
    """Parses bitnum numbers of bit width `bw`
    from the array at the given `path`."""

    def to_decimal(hex_v: str) -> int:
        # Takes in a value in string
        # hexadecimal form, and
        # returns the corresponding
        # integer value.
        v = int(hex_v.strip(), 16)
        if is_signed and v > (2 ** (bw - 1)):
            return -1 * ((2 ** bw) - v)

        return v

    with path.open('r') as f:
        return np.array(
            list(
                map(to_decimal, f.readlines())
            )
        )


def parse_dat_fp(path, width, int_width, is_signed):
    """Parses a fixed point number."""
    to_decimal = lambda x: fp_to_decimal(
        np.binary_repr(
            int(x.strip(), 16),
            width
        ),
        width,
        int_width,
        is_signed
    )

    with path.open('r') as f:
        return np.array(
            list(
                map(to_decimal, f.readlines())
            )
        )


# Go through the json data and create a file for each key,
# flattening the data and then converting it to bitstrings.
def convert2dat(output_dir, data, extension):
    output_dir = Path(output_dir)
    shape = {}
    for k, item in data.items():
        path = output_dir / f"{k}.{extension}"
        path.touch()
        arr = np.array(item["data"])

        numeric_type = item["numeric_type"]
        is_signed = item["is_signed"]
        width = item["width"]

        shape[k] = {
            "shape": list(arr.shape),
            "numeric_type": numeric_type,
            "is_signed": is_signed,
            "width": width
        }
        if numeric_type not in {'bitnum', 'fixed_point'}:
            raise Exception("Give a valid numeric type input.")

        # Verify an integer width is provided for fixed point.
        is_fp = (numeric_type == 'fixed_point')
        if is_fp:
            int_width = item.get("int_width")
            if int_width is None:
                raise Exception(
                    "Fixed point requires a width and integer "
                    "width. The fractional width is inferred."
                )
            shape[k]["int_width"] = int_width

        convert = lambda x: decimal_to_fp(
            x,
            width,
            int_width,
            is_signed
        ) if is_fp else x

        with path.open('w') as f:
            for v in arr.flatten():
                f.write(
                    bitstring(
                        convert(v),
                        width
                    ) + "\n"
                )

    # Commit shape.json file.
    shape_json_file = output_dir / "shape.json"
    with shape_json_file.open('w') as f:
        json.dump(shape, f, indent=2)


# Converts a directory of *.dat files back into a json file.
def convert2json(input_dir, extension):
    input_dir = Path(input_dir)
    data = {}
    shape_json_path = input_dir / "shape.json"
    shape_json = None
    if shape_json_path.exists():
        shape_json = json.load(shape_json_path.open('r'))

    # TODO: change to use shape json
    for val in input_dir.glob(f'*.{extension}'):
        key = val.stem
        stem = shape_json[key]

        numeric_type = stem["numeric_type"]
        is_signed = stem["is_signed"]
        width = stem["width"]

        if numeric_type == 'bitnum':
            arr = parse_dat_bitnum(val, width, is_signed)
        elif numeric_type == 'fixed_point':
            int_width = stem.get("int_width")
            if int_width is None:
                raise Exception(
                    "Fixed point requires a width and integer width. "
                    "The fractional width is inferred."
                )
            arr = parse_dat_fp(
                val,
                width,
                int_width,
                is_signed
            )
        else:
            raise Exception(
                "A valid numeric type is required."
            )

        if shape_json.get(key) is not None and shape_json[key]["shape"] != [0]:
            try:
                arr = arr.reshape(tuple(shape_json[key]["shape"]))
            except Exception:
                raise Exception(f"Key '{key}' had invalid shape.")
        data[key] = arr.tolist()
    return data
