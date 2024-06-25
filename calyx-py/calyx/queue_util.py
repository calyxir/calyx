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
    ranks = None
    try:
        ranks = data["ranks"]["data"]
    except KeyError:
        pass
    return commands, values, ranks


def dump_json(ans_mem, commands, values, ranks=None):
    """Prints a JSON representation of the data to stdout."""
    payload = {}
    if ranks == None:
        payload = {
            "ans_mem": ans_mem,
            "commands": commands,
            "values": values
        }
    else:
        payload = {
            "ans_mem": ans_mem,
            "commands": commands,
            "ranks": ranks,
            "values": values
        }
    print(json.dumps(payload, indent=2))
