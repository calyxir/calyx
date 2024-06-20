from dataclasses import dataclass
from typing import List
from typing import Optional
from calyx import queue_util


class QueueError(Exception):
    """An error that occurs when we try to pop/peek from an empty queue,"""


@dataclass
class Fifo:
    """A FIFO data structure.
    Supports the operations `push`, `pop`, and `peek`.
    Inherent to the queue is its `max_len`,
    which is given at initialization and cannot be exceeded.

    If initialized with "error mode" turned on, the queue raises errors in case
    of underflow or overflow and stops the simulation.
    Otherwise, it allows those commands to fail silently but continues the simulation.
    """

    def __init__(self, data: List[int], error_mode=True, max_len: int = None):
        self.data = data
        self.max_len = max_len or queue_util.QUEUE_SIZE
        self.error_mode = error_mode

    def push(self, val: int) -> None:
        """Pushes `val` to the FIFO."""
        if len(self.data) == self.max_len:
            if self.error_mode:
                raise QueueError("Cannot push to full FIFO.")
            return
        self.data.append(val)

    def pop(self) -> Optional[int]:
        """Pops the FIFO."""
        if len(self.data) == 0:
            if self.error_mode:
                raise QueueError("Cannot pop from empty FIFO.")
            return None
        return self.data.pop(0)

    def peek(self) -> Optional[int]:
        """Peeks into the FIFO."""
        if len(self.data) == 0:
            if self.error_mode:
                raise QueueError("Cannot peek into empty FIFO.")
            return None
        return self.data[0]

    def __len__(self) -> int:
        return len(self.data)


@dataclass
class Pifo:
    """A PIFO data structure.
    Supports the operations `push`, `pop`, and `peek`.

    We do this by maintaining two sub-queues that are given to us at initialization.
    We toggle between these sub-queues when popping/peeking.
    We have a variable called `hot` that says which sub-queue is to be
    popped/peeked next.
    `hot` starts at 0.
    We also take at initialization a `boundary` value.

    We maintain internally a variable called `pifo_len`:
    the sum of the lengths of the two queues.

    Inherent to the queue is its `max_len`, which is given to us at initialization
    and we cannot exceed.

    If initialized with "error mode" turned on, the queue raises errors in case
    of underflow or overflow and stops the simulation.
    Otherwise, it allows those commands to fail silently but continues the simulation.

    When asked to pop:
    - If `pifo_len` is 0, we fail silently or raise an error.
    - Else, if `hot` is 0, we try to pop from queue_0.
      + If it succeeds, we flip `hot` to 1 and return the value we got.
      + If it fails, we pop from queue_1 and return the value we got.
        We leave `hot` as it was.
    - If `hot` is 1, we proceed symmetrically.
    - We decrement `pifo_len` by 1.

    When asked to peek:
    We do the same thing as above, except:
    - We peek into the sub-queue instead of popping it.
    - We don't flip `hot`.

    When asked to push:
    - If the PIFO is at length `max_len`, we fail silently or raise an error.
    - If the value to be pushed is less than `boundary`, we push it into queue_1.
    - Else, we push it into queue_2.
    - We increment `pifo_len` by 1.
    """

    def __init__(self, queue_1, queue_2, boundary, error_mode=True, max_len=None):
        self.data = (queue_1, queue_2)
        self.hot = 0
        self.pifo_len = len(queue_1) + len(queue_2)
        self.boundary = boundary
        self.max_len = max_len or queue_util.QUEUE_SIZE
        self.error_mode = error_mode
        assert (
            self.pifo_len <= self.max_len
        )  # We can't be initialized with a PIFO that is too long.

    def push(self, val: int):
        """Pushes `val` to the PIFO."""
        if self.pifo_len == self.max_len:
            if self.error_mode:
                raise QueueError("Cannot push to full PIFO.")
            return
        if val <= self.boundary:
            self.data[0].push(val)
        else:
            self.data[1].push(val)
        self.pifo_len += 1

    def pop(self) -> Optional[int]:
        """Pops the PIFO."""
        if self.pifo_len == 0:
            if self.error_mode:
                raise QueueError("Cannot pop from empty PIFO.")
            return None
        self.pifo_len -= 1  # We decrement `pifo_len` by 1.
        if self.hot == 0:
            try:
                self.hot = 1
                return self.data[0].pop()
            except QueueError:
                self.hot = 0
                return self.data[1].pop()
        else:
            try:
                self.hot = 0
                return self.data[1].pop()
            except QueueError:
                self.hot = 1
                return self.data[0].pop()

    def peek(self) -> Optional[int]:
        """Peeks into the PIFO."""
        if self.pifo_len == 0:
            if self.error_mode:
                raise QueueError("Cannot peek into empty PIFO.")
            return None
        if self.hot == 0:
            try:
                return self.data[0].peek()
            except QueueError:
                return self.data[1].peek()
        else:
            try:
                return self.data[1].peek()
            except QueueError:
                return self.data[0].peek()

    def __len__(self) -> int:
        return self.pifo_len


@dataclass
class Pieo:
    """A PIEO data structure.
    Supports the operations `push`, `pop`, and `peek`.

    Stores elements ordered increasingly by a totally ordered `rank` attribute (for
    simplicitly, our implementation is just using integers).

    At initialization we take in a set of `(int, int)` pairs `data` which stores
    values and their ranks, and is ordered.
    
    We also take at initialization a `max_len` value to store the maximum possible
    length of a queue.

    If initialized with "error mode" turned on, the queue raises errors in case
    of underflow or overflow and stops the simulation.

    Otherwise, it allows those commands to fail silently but continues the simulation.

    When asked to push:
    - If the PIEO is at length `max_len`,
        we fail silently or raise an error depending on error_mode.
    - Otherwise, we insert the element into the PIEO such that the rank order stays increasing.

    When asked to pop:
    - If the length of `data` is 0,
        we fail silently or raise an error depending on error_mode.

    - We can either pop based on value or based on eligibility.
    - This implementation supports the most common eligibility predicate:
        the <= relation on some bound.

    - If a value is passed in, we pop the first (lowest-rank) instance of that value.
    - If no value is passed in but a bound is,
        we pop the first (lowest-rank) value that passes the predicate.
    - Note that either a value or a bound must be passed in - both cannot be, nor can neither.

    When asked to peek:
    We do the same thing as `pop`, except:
    - We peek into the PIEO instead of popping it - i.e. we don't remove any elements.

    We compactly represent these similar operations through `query`, which takes in an additional
    optional `remove` parameter (defaulted to False) to determine whether to pop or peek.
    """
    
    def __init__(self, data: List[(int, int)], error_mode=True, max_len: int = None):
        """Initialize structure. Ensures that rank ordering is preserved."""
        self.data = data.sort(lambda x : x[1])
        self.error_mode = error_mode
        self.max_len = max_len or queue_util.QUEUE_SIZE
    
    def push(self, val: int, rank : int) -> None:
        """Pushes to a PIEO.
        Inserts element such that rank ordering is preserved
        """
        if len(self.data) == self.max_len:
            if self.error_mode:
                raise QueueError("Cannot push to full PIEO")
            return
        
        else:
            for x in range(len(self.data)):
                if self.ranks[x] >= rank:
                    continue
                else:
                    self.data.insert(x, (val, rank))
    
    def query(self, val=None, bound=None, remove=False) -> Optional[int]:
        """Queries a PIEO. Pops the PIEO if remove is True. Peeks otherwise.
        Can take in either a value or a bound. If a value is passed in,
        query scans data for that value and returns the first instance
        (with the lowest rank).

        If a bound parameter is passed in, this is treated as a parameter for
        an eligibility predicate for which data[i] <= bound.
        """

        if len(self.data) == 0:
            if self.error_mode:
                raise QueueError("Cannot pop from empty PIEO.")
            
        if val == None and bound == None:
            raise QueueError("Either a value or predicate must be supplied.")
        
        elif val != None and bound != None:
            raise QueueError("Cannot supply both a value and an eligibility predicate.")
        
        if bound == None:
            for x in range(len(self.data)):
                if self.data[x][0] == val:
                    return self.data.pop(x)[0] if remove else self.data[x][0]
            return None
        
        if val == None:
            try:
                return [x for x in self.data if x[1] <= bound][0]
            except IndexError:
                raise QueueError("No elements match eligibility predicate")
    
    def pop(self, val=None, bound=None) -> Optional[int]:
        """Pops a PIEO. See query() for specifics."""

        return self.query(val, bound, remove=True)

    def peek(self, val=None, bound=None) -> Optional[int]:
        """Peeks a PIEO. See query() for specifics."""

        return self.query(val, bound, remove=False)

@dataclass
class CalendarQueue:

    def __init__(self, data : List[int], initial_day=0, error_mode=True, max_len: int = None):
        self.data = data
        self.day = initial_day
        self.error_mode = error_mode
        self.max_len = max_len
    
    def push(self, val: int, rank: int) -> None:
        """Pushes a value with some rank/priority to a calendar queue"""
        pass
    
    def pop(self) -> Optional[int]:
        """Pops a calendar queue."""
        pass
    
    def peek(self) -> Optional[int]:
        """Peeks a calendar queue."""
        pass

    def rotate(self) -> None:
        """Rotates a calendar queue"""
        pass


def operate_queue(commands, values, queue):
    """Given the two lists, one of commands and one of values.
    Feed these into our queue, and return the answer memory.
    """

    ans = []
    for cmd, val in zip(commands, values):
        if cmd == 0:
            try:
                result = queue.pop()
                if result:
                    ans.append(result)
            except QueueError:
                break

        elif cmd == 1:
            try:
                result = queue.peek()
                if result:
                    ans.append(queue.peek())
            except QueueError:
                break

        elif cmd == 2:
            try:
                queue.push(val)
            except QueueError:
                break

    # Pad the answer memory with zeroes until it is of length MAX_CMDS.
    ans += [0] * (queue_util.MAX_CMDS - len(ans))
    return ans

def operate_pieo(commands, values, ranks, bounds, queue):
    """Operate a PIEO queue
    Given the four lists:
    - One of commands, one of values, one of ranks, one of bounds:
    - Feed these into our queue, and return the answer memory.
    - Commands correspond to:
        0 : pop by value
        1 : pop by predicate
        2 : peek by value
        3 : peek by predicate
        4 : push
    """

    ans = []
    for cmd, val, rank, bound in zip(commands, values, ranks, bounds):
        if cmd < 2:
            try:
                result = queue.pop(val if cmd == 0 else bound)
                if result:
                    ans.append(result)
            except QueueError:
                break

        elif cmd < 4:
            try:
                result = queue.peek(val if cmd == 2 else bound)
                if result:
                    ans.append(result)
            except QueueError:
                break

        elif cmd == 4:
            try:
                queue.push((val, rank))
            except QueueError:
                break

    # Pad the answer memory with zeroes until it is of length MAX_CMDS.
    ans += [0] * (queue_util.MAX_CMDS - len(ans))
    return ans