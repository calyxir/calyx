import queues
import fifo_oracle

ANS_MEM_LEN = 10


def operate_pifo(commands, values):
    """Given the three lists, operate a PIFO routine.
    In this case, we have our PIFO just orchestrate two FIFOs.
    In the end, we return the answer memory.
    """

    pifo = queues.Pifo(queues.Fifo([]), queues.Fifo([]))
    # Our PIFO is simple: it just orchestrates two FIFOs.

    ans = []
    for cmd, val in zip(commands, values):
        if cmd == 0:
            try:
                ans.append(pifo.pop())
            except IndexError:
                break

        elif cmd == 1:
            try:
                ans.append(pifo.peek())
            except IndexError:
                break

        elif cmd == 2:
            pifo.push(val)

    # Pad the answer memory with zeroes until it is of length ANS_MEM_LEN.
    ans += [0] * (ANS_MEM_LEN - len(ans))
    return ans


if __name__ == "__main__":
    commands, values = fifo_oracle.parse_json()
    ans = operate_pifo(commands, values)
    fifo_oracle.dump_json(commands, values, ans)
