import json
import numpy as np
from pathlib import Path


# Converts `val` into a bitstring that is `bw` characters wide
def bitstring(val, bw):
    # first truncate val by `bw` bits
    val &= 2 ** bw - 1
    return "{:x}".format(val)


def parse_dat(path):
    with path.open("r") as f:
        lines = f.readlines()
        arr = np.array(list(map(lambda v: int(v.strip(), 16), lines)))
        return arr


# go through the json data and create a file for each key,
# flattening the data and then converting it to bitstrings
def convert2dat(output_dir, data, extension):
    output_dir = Path(output_dir)
    shape = {}
    for k, item in data.items():
        path = output_dir / f"{k}.{extension}"
        path.touch()
        arr = np.array(item["data"])
        shape[k] = {"shape": list(arr.shape), "bitwidth": item["bitwidth"]}
        with path.open("w") as f:
            for v in arr.flatten():
                f.write(bitstring(v, item["bitwidth"]) + "\n")

    # commit shape.json file
    shape_json_file = output_dir / "shape.json"
    with shape_json_file.open("w") as f:
        json.dump(shape, f, indent=2)


# converts a directory of *.dat files back into a json file
def convert2json(input_dir, extension):
    input_dir = Path(input_dir)
    data = {}
    shape_json_path = input_dir / "shape.json"
    shape_json = None
    if shape_json_path.exists():
        shape_json = json.load(shape_json_path.open("r"))

    # TODO: change to use shape json
    for f in input_dir.glob(f"*.{extension}"):
        arr = parse_dat(f)
        if (
            shape_json is not None
            and shape_json.get(f.stem) is not None
            and shape_json[f.stem]["shape"] != [0]
        ):
            try:
                arr = arr.reshape(tuple(shape_json[f.stem]["shape"]))
            except Exception:
                raise Exception(f.stem)
            name = f.stem
            data[name] = arr.tolist()
    return data
