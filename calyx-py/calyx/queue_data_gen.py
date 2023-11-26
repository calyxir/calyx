# Usage:
# To make a .data file:
# python calyx-py/calyx/queue_data_gen.py --piezo > calyx-py/test/correctness/sdn.data
# To then make a .expect file:
# cat calyx-py/test/correctness/sdn.data |
# python calyx-py/calyx/pifotree_oracle.py > calyx-py/test/correctness/sdn.expect

import random
import json
import sys
from typing import Dict, Union
from calyx import queue_util

FormatType = Dict[str, Union[bool, str, int]]


def format_gen(width: int) -> FormatType:
    """Generates a format object for a bitvector of length `width`."""
    return {"is_signed": False, "numeric_type": "bitnum", "width": width}


def piezo_special():
    """A special data-gen helper that creates a commands list while
    ensuring that there are:
    - No overflows.
    - No underflows.
    - (queue_util.MAX_CMDS/2) pushes.
    - As many pops as there are pushes.
    A combination of the above means that no packet is left unpopped.
    """
    running_count = 0  # The current size of the queue.
    push_goal = int(queue_util.MAX_CMDS / 2)  # How many pushes we want overall.
    total_push_count = 0
    total_pop_count = 0
    commands = []
    while True:
        command = random.choice(["push", "pop"])
        if command == "pop" and running_count == 0:
            # This would make us underflow,
            # so we'll change the command to `push` instead
            command = "push"
        if command == "push" and running_count == queue_util.QUEUE_SIZE:
            # This would make us overflow,
            # so we'll change the command to `pop` instead
            command = "pop"
        if command == "push":
            running_count += 1
            total_push_count += 1
        if command == "pop":
            running_count -= 1
            total_pop_count += 1
        # Put the command into `commands`.
        commands.append(0 if command == "pop" else 2)

        if total_push_count == push_goal:
            # Pad the `commands` list with (push_goal - total_pop_count) `pop`s,
            # and then break.
            commands += (push_goal - total_pop_count) * [0]
            break

    assert len(commands) == queue_util.MAX_CMDS
    return commands


def dump_json(piezo: bool):
    """Prints a JSON representation of the data to stdout.
    The data itself is populated randomly, following certain rules:
    - It has three "memories": `commands`, `values`, and `ans_mem`.
    - The `commands` memory has queue_util.MAX_CMDS items, which are 0, 1, or 2.
      0: pop, 1: peek, 2: push
      If the `piezo` flag is set, then items are chosen from 0 and 2 using a helper.
    - The `values` memory has queue_util.MAX_CMDS items:
    random values between 0 and 400.
    - The `ans_mem` memory has queue_util.MAX_CMDS items, all zeroes.
    - Each memory has a `format` field, which is a format object for a bitvector.
    """
    commands = {
        "commands": {
            "data": (
                piezo_special()
                if piezo
                else [random.randint(0, 2) for _ in range(queue_util.MAX_CMDS)]
            ),
            "format": format_gen(2),
        }
    }
    values = {
        "values": {
            "data": [random.randint(1, 400) for _ in range(queue_util.MAX_CMDS)],
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
    # Accept a flag that we pass to dump_json.
    # This says whether we should have any 1s in the `commands` memory.

    piezo = len(sys.argv) > 1 and sys.argv[1] == "--piezo"
    random.seed(5)
    dump_json(piezo)
