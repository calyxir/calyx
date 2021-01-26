#!/usr/bin/env python3

import json
import numpy as np
import sys
import itertools
from pathlib import Path


def generate_bank_strings(banks):
    expand = [list(range(x)) for x in banks]
    product = itertools.product(*expand)
    result = []
    for tup in product:
        result.append("_".join(map(str, tup)))
    return result


def generate(size, bitwidth):
    return {"data": np.random.randint(100, size=size).tolist(), "bitwidth": bitwidth}


def modulate_size(size, banks):
    if len(size) == len(banks):
        return (np.array(size) // np.array(banks)).tolist()

    return [0]


def replace(mapping, key):
    if type(key) == int:
        return key

    return mapping[key]


def main(path, all_random):
    template = json.load(path.open())
    mapping = template["key"]
    memory = template["memory"]
    result = {}
    for key in memory:
        size = [replace(mapping, key) for key in memory[key]["data"]]
        banks = [replace(mapping, key) for key in memory[key]["banks"]]
        # include empty string so that we have the empty variant
        variants = [""]
        if "variants" in memory[key]:
            variants += memory[key]["variants"]
        bitwidth = memory[key]["bitwidth"]
        data = generate(modulate_size(size, banks), bitwidth)
        for var in variants:
            # result[f'{key}{var}'] = data # include unbanked for Dahlia
            for b in generate_bank_strings(banks):
                if all_random:
                    result[f"{key}{var}{b}"] = generate(
                        modulate_size(size, banks), bitwidth
                    )
                else:
                    result[f"{key}{var}{b}"] = data
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    filename = Path(sys.argv[1])
    random = False
    if len(sys.argv) > 2:
        random = True
    if filename.exists():
        main(filename, random)
    else:
        print(f"{filename} doesn't exist.")
        exit(1)
