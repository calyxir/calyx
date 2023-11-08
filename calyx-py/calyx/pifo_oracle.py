import fifo_oracle


def operate_pifo(commands, values):
    """Given the three lists, operate a PIFO routine.
    We do this by maintaining two FIFOs and toggling between them when popping.
    We have a variable called `hot` that indicates which FIFO is to be popped next.
    `hot` starts at 1.

    - Read the commands list in order.
    - When the value is 0, we "pop" the PIFO and write the value to the answer memory.
        + This is a little complicated since we are actually popping from two FIFOs.
        + If `len(FIFO_1) + len(FIFO_2)` = 0, break.
        + Try `pop(FIFO_{hot})`.
            * If it succeeds it will return a value `v`; just put `v` in
            the answer memory.
            Also flip `hot` so it points to the other sub-queue.
            * If it fails because of underflow, return `pop(queue_{not-hot})`.
            Leave `hot` as it was.
    - When it is 1, we "peek" into the PIFO and write the value to the answer memory.
    - When it is 2, we push the coressponding item in the `values` list into
    one of our two FIFOs.
        + In particular, if the value is less than 200, it goes into the first FIFO.
        + If it is greater than 200, it goes into the second FIFO.

    In the end, we return the answer memory.
    """
    fifo_1 = []
    fifo_2 = []
    ans = []
    hot = 1
    for cmd, val in zip(commands, values):
        pifo_len = len(fifo_1) + len(fifo_2)
        if cmd == 0:
            if pifo_len == 0:
                break
            # We have to pop from the PIFO.
            if hot == 1:
                try:
                    ans.append(fifo_1.pop(0))  # Suceess.
                    hot = 2  # Flip hot.
                except IndexError:
                    ans.append(fifo_2.pop(0))  # Recovery. Leave hot as it was.
            else:
                try:
                    ans.append(fifo_2.pop(0))  # Suceess.
                    hot = 1  # Flip hot.
                except IndexError:
                    ans.append(fifo_1.pop(0))  # Recovery. Leave hot as it was.
        elif cmd == 1:
            if pifo_len == 0:
                break
            # We have to peek into the PIFO.
            if hot == 1:
                try:
                    ans.append(fifo_1[0])  # Suceess.
                except IndexError:
                    ans.append(fifo_2[0])  # Recovery.
            else:
                try:
                    ans.append(fifo_2[0])  # Suceess.
                except IndexError:
                    ans.append(fifo_1[0])  # Recovery.
        elif cmd == 2:
            # We have to push into the PIFO.
            if val < 200:
                fifo_1.append(val)
            else:
                fifo_2.append(val)

    # Pad the answer memory with zeroes until it is of length ANS_MEM_LEN.
    ans += [0] * (10 - len(ans))
    return ans


if __name__ == "__main__":
    commands, values = fifo_oracle.parse_json()
    ans = operate_pifo(commands, values)
    fifo_oracle.dump_json(commands, values, ans)
