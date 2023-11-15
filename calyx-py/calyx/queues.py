from dataclasses import dataclass
from typing import List
import calyx.queue_util as queue_util


@dataclass
class Fifo:
    """A FIFO data structure.
    Supports the operations `push`, `pop`, and `peek`.
    Inherent to the queue is its `max_len`, which is given to us at initialization
    and we cannot exceed.
    """

    def __init__(self, data: List[int], max_len: int = None):
        self.data = data
        self.max_len = max_len or queue_util.QUEUE_SIZE

    def push(self, val: int):
        """Pushes `val` to the FIFO."""
        if len(self.data) < self.max_len:
            self.data.append(val)
        else:
            raise IndexError("Cannot push to full FIFO.")

    def pop(self) -> int:
        """Pops the FIFO."""
        if len(self.data) == 0:
            raise IndexError("Cannot pop from empty FIFO.")
        return self.data.pop(0)

    def peek(self) -> int:
        """Peeks into the FIFO."""
        if len(self.data) == 0:
            raise IndexError("Cannot peek into empty FIFO.")
        return self.data[0]

    def __len__(self) -> int:
        return len(self.data)


@dataclass
class Pifo:
    """A PIFO data structure.
    Supports the operations `push`, `pop`, and `peek`.

    We do this by maintaining two queues that are given to us at initialization.
    We toggle between these queues when popping/peeking.
    We have a variable called `hot` that says which queue is to be popped/peeked next.
    `hot` starts at 0.
    We also take at initialization a `boundary` value.

    We maintain internally a variable called `pifo_len`:
    the sum of the lengths of the two queues.

    Inherent to the queue is its `max_len`, which is given to us at initialization
    and we cannot exceed.


    When asked to pop:
    - If `pifo_len` is 0, we raise an error.
    - Else, if `hot` is 0, we try to pop from queue_0.
      + If it succeeds, we flip `hot` to 1 and return the value we got.
      + If it fails, we pop from queue_1 and return the value we got.
        We leave `hot` as it was.
    - If `hot` is 1, we proceed symmetrically.
    - We decrement `pifo_len` by 1.

    When asked to peek:
    We do the same thing as above, except:
    - We peek instead of popping.
    - We don't flip `hot`.

    When asked to push:
    - If the PIFO is at length `max_len`, we raise an error.
    - If the value to be pushed is less than `boundary`, we push it into queue_1.
    - Else, we push it into queue_2.
    - We increment `pifo_len` by 1.
    """

    def __init__(self, queue_1, queue_2, boundary, max_len=None):
        self.data = (queue_1, queue_2)
        self.hot = 0
        self.pifo_len = len(queue_1) + len(queue_2)
        self.boundary = boundary
        self.max_len = max_len or queue_util.QUEUE_SIZE
        assert (
            self.pifo_len <= self.max_len
        )  # We can't be initialized with a PIFO that is too long.

    def push(self, val: int):
        """Pushes `val` to the PIFO."""
        if self.pifo_len == self.max_len:
            raise IndexError("Cannot push to full PIFO.")
        if val < self.boundary:
            self.data[0].push(val)
        else:
            self.data[1].push(val)
        self.pifo_len += 1

    def pop(self) -> int:
        """Pops the PIFO."""
        if self.pifo_len == 0:
            raise IndexError("Cannot pop from empty PIFO.")
        self.pifo_len -= 1  # We decrement `pifo_len` by 1.
        if self.hot == 0:
            try:
                self.hot = 1
                return self.data[0].pop()
            except IndexError:
                return self.data[1].pop()
        else:
            try:
                self.hot = 0
                return self.data[1].pop()
            except IndexError:
                return self.data[0].pop()

    def peek(self) -> int:
        """Peeks into the PIFO."""
        if self.pifo_len == 0:
            raise IndexError("Cannot peek into empty PIFO.")
        if self.hot == 0:
            try:
                return self.data[0].peek()
            except IndexError:
                return self.data[1].peek()
        else:
            try:
                return self.data[1].peek()
            except IndexError:
                return self.data[0].peek()

    def __len__(self) -> int:
        return self.pifo_len


def operate_queue(commands, values, queue):
    """Given the two lists, one of commands and one of values.
    Feed these into our queue, and return the answer memory.
    """

    ans = []
    for cmd, val in zip(commands, values):
        if cmd == 0:
            try:
                ans.append(queue.pop())
            except IndexError:
                break

        elif cmd == 1:
            try:
                ans.append(queue.peek())
            except IndexError:
                break

        elif cmd == 2:
            try:
                queue.push(val)
            except IndexError:
                break

    # Pad the answer memory with zeroes until it is of length MAX_CMDS.
    ans += [0] * (queue_util.MAX_CMDS - len(ans))
    return ans
