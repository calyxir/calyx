import sys
import json
import argparse
from .parse import parse
from .gen_futil import emit, emit_data
from .interp import interp, InterpError


def main():
    parser = argparse.ArgumentParser(
        "Interpret a MrXL program, or compile it to Calyx."
    )
    parser.add_argument(
        "--i",
        "--interpret",
        action="store_true",
        help="Interpret the input MrXL program (leave this off to compile)",
    )
    parser.add_argument(
        "--data",
        metavar="<datafile>",
        type=str,
        help="Input data, required to interpret",
    )
    parser.add_argument(
        "--convert",
        action="store_true",
        help="Convert <datafile> to calyx input (instead of compiling)",
    )
    parser.add_argument(
        "filename",
        metavar="<file>",
        type=str,
        help="MrXL program to compile or interpet",
    )

    args = parser.parse_args()
    with open(args.filename) as f:
        txt = f.read()

    if args.data:
        with open(args.data) as f:
            data = json.load(f)

    ast = parse(txt)

    if args.i:
        try:
            print(interp(ast, data))
        except InterpError as exc:
            print(str(exc), file=sys.stderr)
            sys.exit(1)
    elif args.convert:
        if not args.data:
            raise ValueError("--convert was passed but --data not given")
        emit_data(ast, data)
    else:
        emit(ast)

    sys.exit(0)
