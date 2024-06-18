import json
import sys


def parse_json():
    """Effectively the opposite of `data_gen`:
    Given a JSON file formatted for Calyx purposes, parses it into two lists:
    - The `commands` list.
    - The `values` list.
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
