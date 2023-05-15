import sys
import json
import argparse
from .parse import parse
from .gen_calyx import emit
from .interp import interp, InterpError


def main():
    """Main entry point for the MrXL interpreter and compiler."""

    parser = argparse.ArgumentParser(
        "Either interpret a MrXL program, or compile it to Calyx"
    )
    parser.add_argument(
        "--i",
        "--interpret",
        action="store_true",
        help="Interpret the input MrXL program (drop this flag in order to compile)",
    )
    parser.add_argument(
        "--data",
        metavar="<datafile>",
        type=str,
        help="Input data, required if interpreting",
    )
    parser.add_argument(
        "filename",
        metavar="<file>",
        type=str,
        help="The MrXL program to interpret/compile",
    )

    args = parser.parse_args()
    with open(args.filename, encoding="UTF-8") as file:
        txt = file.read()

    if args.data:
        with open(args.data, encoding="UTF-8") as file:
            data = json.load(file)

    ast = parse(txt)

    if args.i:
        if not args.data:
            raise ValueError("Must provide data if interpreting")
        try:
            print(interp(ast, data))  # type: ignore
        except InterpError as exc:
            print(str(exc), file=sys.stderr)
            sys.exit(1)
    else:
        emit(ast)

    sys.exit(0)
