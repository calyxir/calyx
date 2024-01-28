"""Convert between fud-style JSON and hex data files.

Use the machinery from "old fud" to convert a JSON data file into a
directory of flat hex-encoded files, suitable for loading into a
hardware simulator, and back again.
"""
from fud.stages.verilator.json_to_dat import convert2dat, convert2json
import simplejson
import sys
import os
import re


def json2dat(in_file, out_dir):
    os.makedirs(out_dir, exist_ok=True)
    round_float_to_fixed = True
    with open(in_file) as json:
        convert2dat(
            out_dir,
            simplejson.load(json, use_decimal=True),
            "dat",
            round_float_to_fixed,
        )


def dat2json(out_file, in_dir, sim_log=None):
    mem = convert2json(in_dir, "out")

    if sim_log:
        cycles = 0
        with open(sim_log) as f:
            for line in f:
                match = re.search(r"Simulated\s+((-)?\d+) cycles", line)
                if match:
                    cycles = int(match.group(1))
                    break
        out = {
            "cycles": cycles,
            "memories": mem,
        }
    else:
        out = mem

    with open(out_file, 'w') as f:
        simplejson.dump(out, f, indent=2, sort_keys=True, use_decimal=True)


if __name__ == '__main__':
    if sys.argv[1] == '--from-json':
        json2dat(*sys.argv[2:])
    elif sys.argv[1] == '--to-json':
        dat2json(*sys.argv[2:])
    else:
        print("specify --from-json or --to-json", file=sys.stderr)
