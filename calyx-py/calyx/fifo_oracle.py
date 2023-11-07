import sys
import json


def parse_json():
    """Effectively the opposite of `data_gen`:
    Given a JSON file formatted for Calyx purposes, parse it into its two lists:
    - The `commands` array, which has MAX_CMDS items.
    - The `values` array, which has MAX_CMDS items.

    Returns the three lists.
    """

    # The JSON file is piped to us in stdin.
    data = json.load(sys.stdin)
    commands = data["commands"]["data"]
    values = data["values"]["data"]
    return commands, values


def operate_fifo(commands, values):
    """Given the three lists, operate a FIFO routine.
    - Read the comammands list in order.
    - When the value is 0, we "pop" the FIFO and write the value to the answer memory.
    - When it is 1, we "peek" into the FIFO and write the value to the answer memory.
    - When it is 2, we push the coressponding item in the `values` list to the FIFO.

    In the end, we return the answer memory.
    """
    fifo = []
    ans = []
    for cmd, val in zip(commands, values):
        if cmd == 0:
            if len(fifo) == 0:
                break
            ans.append(fifo.pop(0))
        elif cmd == 1:
            if len(fifo) == 0:
                break
            ans.append(fifo[0])
        elif cmd == 2:
            fifo.append(val)
    # Pad the answer memory with zeroes until it is of length ANS_MEM_LEN.
    ans += [0] * (10 - len(ans))
    return ans


def dump_json(commands, values, ans_mem):
    payload = {
        "ans_mem": ans_mem,
        "commands": commands,
        "values": values,
    }
    print(json.dumps(payload, indent=2))


if __name__ == "__main__":
    commands, values = parse_json()
    ans = operate_fifo(commands, values)
    dump_json(commands, values, ans)
