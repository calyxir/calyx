import random
import time
import json
from typing import Dict, Union

MAX_CMDS = 15
ANS_MEM_LEN = 10

FormatType = Dict[str, Union[bool, str, int]]


def format_gen(width: int) -> FormatType:
    """Generates a format object for a bitvector of length `width`."""
    return {"is_signed": False, "numeric_type": "bitnum", "width": width}


def dump_json():
    """Prints a JSON representation of the data to stdout.
    The data itself is populated randomly, following certain rules:
    - It has three "memories": `commands`, `values`, and `ans_mem`.
    - The `commands` memory has MAX_CMDS items, which are 0, 1, or 2.
    - The `values` memory has MAX_CMDS items: random values between 0 and 100.
    - The `ans_mem` memory has ANS_MEM_LEN items, all zeroes.
    - Each memory has a `format` field, which is a format object for a bitvector.
    """
    commands = {
        "commands": {
            "data": [random.randint(0, 2) for _ in range(MAX_CMDS)],
            # The `commands` memory has MAX_CMDS items, which are 0, 1, or 2.
            "format": format_gen(2),
        }
    }
    values = {
        "values": {
            "data": [random.randint(0, 100) for _ in range(MAX_CMDS)],
            # The `values` memory has MAX_CMDS items: random values between 0 and 100.
            "format": format_gen(32),
        }
    }
    ans_mem = {
        "ans_mem": {
            "data": [0 for _ in range(ANS_MEM_LEN)],
            # The `ans_mem` memory has ANS_MEM_LEN items, all zeroes.
            "format": format_gen(32),
        }
    }

    print(json.dumps(commands | values | ans_mem, indent=2))


if __name__ == "__main__":
    random.seed(5)
    dump_json()
