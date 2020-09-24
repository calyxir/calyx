import json
import sys
import argparse
import numpy as np
from pathlib import Path

# Defines cmdline parser
# def make_parser():
#     parser = argparse.ArgumentParser(
#         description='Convert json files into a series of .dat files for reading into Verilog memories'
#     )
#     parser.add_argument(
#         'input',
#         nargs='?',
#         type=str,
#         help='The json file to convert. If excluded, pass in a file over stdin.'
#     )
#     parser.add_argument(
#         '--mode',
#         default='json',
#         choices=['json', 'dat'],
#         help='The `json` mode converts json->{dat} and `dat` converts {dat}->json'
#     )
#     parser.add_argument(
#         '--output',
#         default='output',
#         type=str,
#         help='The directory to put generated files.'
#     )
#     parser.add_argument(
#         '--read_ext',
#         default='out',
#         help='The file extension that `dat` mode looks for when reading dat files.'
#     )
#     parser.add_argument(
#         '--write_ext',
#         default='dat',
#         help='The file extension that `json` mode looks for when writing dat files.'
#     )
#     return parser

# Converts `val` into a bitstring that is `bw` characters wide
def bitstring(val, bw):
    # first truncate val by `bw` bits
    val &= (2**bw - 1)
    return '{:x}'.format(val)

def parse_dat(path):
    with path.open('r') as f:
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
        shape[k] = { "shape": list(arr.shape), "bitwidth": item["bitwidth"] }
        with path.open('w') as f:
            for v in arr.flatten():
                f.write(bitstring(v, item["bitwidth"]) + "\n")

    # commit shape.json file
    shape_json_file = output_dir / "shape.json"
    with shape_json_file.open('w') as f:
        json.dump(shape, f, indent=2)

# converts a directory of *.dat files back into a json file
def convert2json(input_dir, extension):
    input_dir = Path(input_dir)
    data = {}
    shape_json_path = input_dir / "shape.json"
    shape_json = None
    if shape_json_path.exists():
        shape_json = json.load(shape_json_path.open('r'))

    # TODO: change to use shape json
    for f in input_dir.glob(f'*.{extension}'):
        arr = parse_dat(f)
        if shape_json != None and shape_json.get(f.stem) != None and shape_json[f.stem]["shape"] != [0]:
            try:
                arr = arr.reshape(tuple(shape_json[f.stem]["shape"]))
            except:
                raise Exception(f.stem)
            name = f.stem
            # TODO: this is probably important, figure out why (I think it was used for Dahlia benchmarks)
            # if '_int' in name:
            data[name] = arr.tolist()
    return data

# def main():
#     parser = make_parser()
#     args = parser.parse_args()

#     if args.input != None:
#         if args.mode == 'json':
#             output_dir = Path(args.output)
#             output_dir.mkdir(parents=True, exist_ok=True)

#             # load the json from the file
#             with open(args.input) as f:
#                 convert2dat(output_dir, json.load(f), args.write_ext)
#         elif args.mode == 'dat':
#             convert2json(Path(args.input), Path(args.output), args.read_ext)
#     else:
#         if sys.stdin.isatty():
#             # no input file and nothing piped in over stdin
#             parser.print_help(sys.stderr)
#             sys.exit(1)
#         else:
#             if args.mode == 'json':
#                 output_dir = Path(args.output)
#                 output_dir.mkdir(parents=True, exist_ok=True)
#                 convert2dat(output_dir, json.load(sys.stdin), args.write_ext)
#             elif args.mode == 'dat':
#                 raise Exception("`dat` mode does not work over stdin.")
