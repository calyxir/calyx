import fifo_oracle

from dataclasses import dataclass
from typing import List, Tuple

ANS_MEM_LEN = 10


@dataclass
class Pifo:
    """A PIFO data structure.
    Supports the operations `push`, `pop`, and `peek`.

    We do this by maintaining two FIFOs and toggling between them when popping.
    We have a variable called `hot` that indicates which FIFO is to be popped next.
    `hot` starts at 1.

    We maintain a variable called `pifo_len`: the sum of the lengths of the two FIFOs.

    When asked to pop:
    - If `pifo_len` is 0, we raise an error.
    - Else, if `hot` is 1, we try to pop from FIFO_1.
      + If it succeeds, we flip `hot` to 2 and return the value we got.
      + If it fails, we pop from FIFO_2 and return the value we got.
        We leave `hot` as it was.
    - If `hot` is 2, we proceed symmetrically.
    - We decrement `pifo_len` by 1.

    When asked to peek:
    We do the same thing as above, except:
    - We peek instead of popping.
    - We don't flip `hot`.

    When asked to push:
    - If the value to be pushed is less than 200, we push it into FIFO_1.
    - Else, we push it into FIFO_2.
    - We increment `pifo_len` by 1.
    """

    data = Tuple[fifo_oracle.Fifo, fifo_oracle.Fifo]

    def __init__(self):
        self.data = (fifo_oracle.Fifo([]), fifo_oracle.Fifo([]))
        self.hot = 1
        self.pifo_len = 0

    def push(self, val: int):
        """Pushes `val` to the PIFO."""
        if val < 200:
            self.data[0].push(val)
        else:
            self.data[1].push(val)
        self.pifo_len += 1

    def pop(self) -> int:
        """Pops the PIFO."""
        if self.pifo_len == 0:
            raise IndexError("Cannot pop from empty PIFO.")
        if self.hot == 1:
            try:
                self.pifo_len -= 1
                self.hot = 2
                return self.data[0].pop()
            except IndexError:
                self.pifo_len -= 1
                return self.data[1].pop()
        else:
            try:
                self.pifo_len -= 1
                self.hot = 1
                return self.data[1].pop()
            except IndexError:
                self.pifo_len -= 1
                return self.data[0].pop()

    def peek(self) -> int:
        """Peeks into the PIFO."""
        if self.pifo_len == 0:
            raise IndexError("Cannot peek into empty PIFO.")
        if self.hot == 1:
            try:
                return self.data[0].peek()
            except IndexError:
                return self.data[1].peek()
        else:
            try:
                return self.data[1].peek()
            except IndexError:
                return self.data[0].peek()


def operate_pifo(commands, values):
    """Given the three lists, operate a PIFO routine.
    In the end, we return the answer memory.
    """

    pifo = Pifo()
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
