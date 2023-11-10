import random
import json
from typing import Dict, Union
import calyx.queue_util as queue_util

FormatType = Dict[str, Union[bool, str, int]]


def format_gen(width: int) -> FormatType:
    """Generates a format object for a bitvector of length `width`."""
    return {"is_signed": False, "numeric_type": "bitnum", "width": width}


def dump_json():
    """Prints a JSON representation of the data to stdout.
    The data itself is populated randomly, following certain rules:
    - It has three "memories": `commands`, `values`, and `ans_mem`.
    - The `commands` memory has queue_util.MAX_CMDS items, which are 0, 1, or 2.
    - The `values` memory has queue_util.MAX_CMDS items:
    random values between 0 and 400.
    - The `ans_mem` memory has queue_util.MAX_CMDS items, all zeroes.
    - Each memory has a `format` field, which is a format object for a bitvector.
    """
    commands = {
        "commands": {
            # We'll "rig" these random values a little.
            # The first 20% of the commands will be 2 (push).
            # The rest will be generated randomly from among 0, 1, and 2.
            "data": [2] * (queue_util.MAX_CMDS // 5)
            + [random.randint(0, 2) for _ in range(queue_util.MAX_CMDS * 4 // 5)],
            "format": format_gen(2),
        }
    }
    values = {
        "values": {
            "data": [random.randint(0, 400) for _ in range(queue_util.MAX_CMDS)],
            # The `values` memory has queue_util.MAX_CMDS items: random values
            # between 0 and 400.
            "format": format_gen(32),
        }
    }
    ans_mem = {
        "ans_mem": {
            "data": [0 for _ in range(queue_util.MAX_CMDS)],
            # The `ans_mem` memory has queue_util.MAX_CMDS items, all zeroes.
            "format": format_gen(32),
        }
    }

    print(json.dumps(commands | values | ans_mem, indent=2))


if __name__ == "__main__":
    random.seed(5)
    dump_json()
