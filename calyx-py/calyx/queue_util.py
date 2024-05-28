import json
import sys

MAX_CMDS = 20000
QUEUE_SIZE = 16


def parse_json():
    """Effectively the opposite of `data_gen`:
    Given a JSON file formatted for Calyx purposes, parse it into its two lists:
    - The `commands` memory, which has MAX_CMDS items.
    - The `values` memory, which has MAX_CMDS items.
    Returns the two lists.
    """

    data = json.load(sys.stdin)
    commands = data["commands"]["data"]
    values = data["values"]["data"]
    return commands, values


def dump_json(commands, values, ans_mem):
    """Prints a JSON representation of the data to stdout."""
    payload = {
        "ans_mem": ans_mem,
        "commands": commands,
        "values": values,
    }
    print(json.dumps(payload, indent=2))
