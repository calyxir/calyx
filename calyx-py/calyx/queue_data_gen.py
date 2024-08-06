# For usage, see gen_queue_data_expect.sh

import random

import json
import sys
from typing import Dict, Union, Optional

FormatType = Dict[str, Union[bool, str, int]]


def format_gen(width: int) -> FormatType:
    """Generates a format object for a bitvector of length `width`."""
    return {"is_signed": False, "numeric_type": "bitnum", "width": width}


def no_err_cmds_list(queue_size, num_cmds):
    """A special data-gen helper that creates a commands list while
    ensuring that there are:
    - No overflows.
    - No underflows.
    - `num_cmds`/2 pushes and `num_cmds`/2 pops.
    A combination of the above means that no packet is left unpopped.
    This is specifically catered for work-conserving algorithms,
    and not for non-work-conserving algorithms.
    """
    running_count = 0  # The current size of the queue.
    push_goal = int(num_cmds / 2)  # How many pushes we want overall.
    total_push_count = 0
    total_pop_count = 0
    commands = []
    while True:
        command = random.choice(["push", "pop"])
        if command == "pop" and running_count == 0:
            # This would make us underflow,
            # so we'll change the command to `push` instead
            command = "push"
        if command == "push" and running_count == queue_size:
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
        commands.append(0 if command == "pop" else 1)

        if total_push_count == push_goal:
            # Pad the `commands` list with (push_goal - total_pop_count) `pop`s,
            # and then break.
            commands += (push_goal - total_pop_count) * [0]
            break

    # If the total number of commands is not `num_cmds`, pad it with `peek`s.
    # This is because the `commands` list must have `num_cmds` items.
    commands += (num_cmds - len(commands)) * [1]
    # The above command will add either zero or one `peek` command to the end.

    assert (
        len(commands) == num_cmds
    ), f"Length of commands list was {len(commands)}, expected {num_cmds}"
    return commands


def dump_json(num_cmds, no_err: bool, queue_size: Optional[int]=None, nwc=False, use_ranks=False):
    """Prints a JSON representation of the data to stdout.
    The data itself is populated randomly, following certain rules:
    - It has three "memories": `commands`, `values`, and `ans_mem`.
    - Optional memories `ranks` and `times` are included for queues primed for non-work-conserving algorithms.
    - The `commands` memory has `num_cmds` items, which are 0 or 1 for both work-conserving and 
        non-work-conserving policies. They are as follows:

    FOR WORK-CONSERVING POLICIES
        0 : pop
        1 : push
    
    FOR NON-WORK-CONSERVING POLICIES
        0 : pop by predicate
        1 : push

    If the `no_err` flag is set and the policy is work-conserving,
    then items are chosen from 0 and 2 using a helper.

    If the `nwc` flag is set to False (marking the policy as work-conserving),
    then the predicate is treated as though always true.

    - The `values` memory has `num_cmds` items:
    random values between 0 and 400.
    - The `ranks` memory has `num_cmds` items:
    random values between 0 and 400.
    - The `times` memory has `num_cmds` items:
    random values between 0 and 50.
    - The `ans_mem` memory has `num_cmds` items, all zeroes.
    - Each memory has a `format` field, which is a format object for a bitvector.
    """
        
    commands = {
        "commands": {
            "data": (
                # The `commands` memory has `num_cmds` items, which are all 0 or 1
                no_err_cmds_list(queue_size, num_cmds)
                if no_err
                # If the `no_err` flag is set, then we use the special helper
                # that ensures no overflow or overflow will occur.
                else (
                    [random.randint(0, 1) for _ in range(num_cmds)]
                )
            ),
            "format": format_gen(1),
        }
    }
    values = {
        "values": {
            "data": [random.randint(0, 400) for _ in range(num_cmds)],
            # The `values` memory has `num_cmds` items, which are all
            # random values between 0 and 400.
            "format": format_gen(32),
        }
    }
    ranks = {
        "ranks": {
            "data": [random.randint(0, 400) for _ in range(num_cmds)],
            # The `ranks` memory has `num_cmds` items, which are all
            # random values between 0 and 400.
            "format": format_gen(32),
        }
    }
    times = {
        "times": {
            "data": [0 if not nwc else random.randint(0, 50) for _ in range(num_cmds)],
            # The `times` memory has `num_cmds` items, which are all
            # random values between 0 and 50.
            "format": format_gen(32),
        }        
    }
    ans_mem = {
        "ans_mem": {
            "data": [0] * num_cmds,
            # The `ans_mem` memory has `num_cmds` items, all zeroes.
            "format": format_gen(32),
        }
    }

    if nwc:
        print(json.dumps(commands | values | ranks | times | ans_mem, indent=2))
    elif use_ranks:
        print(json.dumps(commands | values | ranks | ans_mem, indent=2))
    else:
        print(json.dumps(commands | values | ans_mem, indent=2))


if __name__ == "__main__":
    # Accept a flag that we pass to dump_json.
    # This says whether we should use the special no_err helper.
    random.seed(5)
    num_cmds = int(sys.argv[1])
    nwc = "--nwc-en" in sys.argv
    no_err = "--no-err" in sys.argv
    use_rank = "--use-rank" in sys.argv
    if no_err:
        queue_size = int(sys.argv[3])
    dump_json(num_cmds, no_err, queue_size if no_err else None, nwc, use_rank)
