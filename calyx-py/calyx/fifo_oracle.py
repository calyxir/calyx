import sys
import json
import queues

ANS_MEM_LEN = 10


def parse_json():
    """Effectively the opposite of `data_gen`:
    Given a JSON file formatted for Calyx purposes, parse it into its two lists:
    - The `commands` memory, which has MAX_CMDS items.
    - The `values` memory, which has MAX_CMDS items.
    Returns the two lists.
    """

    # The JSON file is piped to us in stdin.
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


def operate_fifo(commands, values):
    """Given the two lists, operate a FIFO routine.
    - Read the commands list in order.
    - When the value is 0, we "pop" the FIFO and write the value to the answer memory.
    - When it is 1, we "peek" into the FIFO and write the value to the answer memory.
    - When it is 2, we push the coressponding item in the `values` list to the FIFO.

    In the end, we return the answer memory.
    """
    fifo = queues.Fifo([])
    ans = []
    for cmd, val in zip(commands, values):
        if cmd == 0:
            try:
                ans.append(fifo.pop())
            except IndexError:
                break

        elif cmd == 1:
            try:
                ans.append(fifo.peek())
            except IndexError:
                break

        elif cmd == 2:
            fifo.push(val)

    # Pad the answer memory with zeroes until it is of length ANS_MEM_LEN.
    ans += [0] * (ANS_MEM_LEN - len(ans))
    return ans


def dump_json(commands, values, ans_mem):
    """Prints a JSON representation of the data to stdout."""
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
