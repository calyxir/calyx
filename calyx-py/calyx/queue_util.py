import json
import sys


def parse_json(parse_ranks=False, parse_times=False):
    """Effectively the opposite of `data_gen`:
    Given a JSON file formatted for Calyx purposes, parses it into two lists:
    - The `commands` list.
    - The `values` list.
    Returns the two lists.
    """
    data = json.load(sys.stdin)
    commands = data["commands"]["data"]
    values = data["values"]["data"]

    if parse_ranks:
        ranks = data["ranks"]["data"]
    if parse_times:
        times = data["times", "data"]
        return commands, values, ranks, times if parse_ranks else commands, values, times
    
    return commands, values


def dump_json(commands, values, ans_mem, ranks=None, times=None):
    """Prints a JSON representation of the data to stdout."""

    payload = {
        "ans_mem": ans_mem,
        "commands": commands,
        "values": values,
    }

    if ranks:
        payload["ranks"] = ranks
    if times:
        payload["times"] = times
        
    print(json.dumps(payload, indent=2))
