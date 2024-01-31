import simplejson
import sys
import pathlib
from fud.stages.interpreter import convert_to_json, parse_from_json


def data2interp(in_file):
    """Convert a fud-style JSON data file to Cider-ready JSON.

    The output file is hard-coded to be `data.json`.
    """
    round_float_to_fixed = True
    with open(in_file) as f:
        convert_to_json(
            '.',
            simplejson.load(f, use_decimal=True),
            round_float_to_fixed,
        )


def interp2data(in_file, orig_file):
    """Convert the Cider's output JSON to fud-style JSON.

    Print the result to stdout.
    """
    with open(in_file) as f:
        out = parse_from_json(f, pathlib.Path(orig_file))
    simplejson.dump(
        out,
        sys.stdout,
        indent=2,
        sort_keys=True,
        use_decimal=True,
    )


if __name__ == "__main__":
    if sys.argv[1] == '--to-interp':
        data2interp(*sys.argv[2:])
    elif sys.argv[1] == '--from-interp':
        interp2data(*sys.argv[2:])
    else:
        print("specify --to-interp or --from-interp", file=sys.stderr)
