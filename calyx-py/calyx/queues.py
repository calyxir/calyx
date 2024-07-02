from dataclasses import dataclass
from typing import List
from typing import Optional


class QueueError(Exception):
    """An error that occurs when we try to pop/peek from an empty queue,"""

class CmdError(Exception):
    """An error that occurs if an undefined command is passed"""


@dataclass
class Fifo:
    """A FIFO data structure.
    Supports the operations `push`, `pop`, and `peek`.
    Inherent to the queue is its `max_len`,
    which is given at initialization and cannot be exceeded.
    """

    def __init__(self, max_len: int):
        self.data = []
        self.max_len = max_len

    def push(self, val: int, time=0, rank=None) -> None:
        """Pushes `val` to the FIFO."""
        if len(self.data) == self.max_len:
            raise QueueError("Cannot push to full FIFO.")
        self.data.append(val)

    def pop(self, time=0) -> Optional[int]:
        """Pops the FIFO."""
        if len(self.data) == 0:
            raise QueueError("Cannot pop from empty FIFO.")
        return self.data.pop(0)

    def peek(self, time=0) -> Optional[int]:
        """Peeks into the FIFO."""
        if len(self.data) == 0:
            raise QueueError("Cannot peek into empty FIFO.")
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

    def __init__(self, queue_1, queue_2, boundary, max_len):
        self.data = (queue_1, queue_2)
        self.hot = 0
        self.pifo_len = len(queue_1) + len(queue_2)
        self.boundary = boundary
        self.max_len = max_len
        assert (
            self.pifo_len <= self.max_len
        )  # We can't be initialized with a PIFO that is too long.

    def push(self, val: int, time=0, rank=None) -> None:
        """Pushes `val` to the PIFO."""
        if self.pifo_len == self.max_len:
            raise QueueError("Cannot push to full PIFO.")
        if val <= self.boundary:
            self.data[0].push(val)
        else:
            self.data[1].push(val)
        self.pifo_len += 1

    def pop(self, time=0) -> Optional[int]:
        """Pops the PIFO."""
        if self.pifo_len == 0:
            raise QueueError("Cannot pop from empty PIFO.")
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

    def peek(self, time=0) -> Optional[int]:
        """Peeks into the PIFO."""
        if self.pifo_len == 0:
            raise QueueError("Cannot peek into empty PIFO.")
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

    At initialization we take in a set of `(int, int, int)` triples `data` which stores
    values, ready times, and their ranks, and is ordered by rank.
    
    We also take at initialization a `max_len` value to store the maximum possible
    length of a queue.

    When asked to push:
    - If the PIEO is at length `max_len`, we raise an error.
    - Otherwise, we insert the element into the PIEO such that the rank order stays increasing.

    When asked to pop:
    - If the length of `data` is 0, we raise an error .

    - We can either pop based on value or based on eligibility.
    - This implementation supports the most common eligibility predicate - whether an element is 'ripe'.

    - If a value is passed in, we pop the first (lowest-rank) instance of that value.
    - If no value is passed in but a time is,
        we pop the first (lowest-rank) value that passes the predicate.
    - Note that either a value or a bound must be passed in - both cannot be, nor can neither.

    When asked to peek:
    We do the same thing as `pop`, except:
    - We peek into the PIEO instead of popping it - i.e. we don't remove any elements.

    We compactly represent these similar operations through `query`, which takes in an additional
    optional `remove` parameter (defaulted to False) to determine whether to pop or peek.
    """
    
    def __init__(self, max_len: int):
        """Initialize structure."""
        self.max_len = max_len
        self.data = []

    def ripe(self, val, time):
        """Check that a value is 'ripe' – i.e. its ready time has passed"""
        return val[1] <= time
    
    def binsert(self, val, time, rank, l, r):
        """Inserts element into list such that rank ordering is preserved
        Uses variant of binary search algorithm
        """
        if l == r:
            return self.data.insert(l, (val, time, rank))

        mid = (l + r) // 2

        if rank == self.data[mid][2]:
            return self.data.insert(mid, (val, time, rank))

        if rank > self.data[mid][2]:
            return self.binsert(val, time, rank, mid+1, r)

        if rank < self.data[mid][2]:
            return self.binsert(val, time, rank, l, mid)
        
    def push(self, val, time=0, rank=0) -> None:
        """Pushes to a PIEO.
        Inserts element such that rank ordering is preserved
        """
        if len(self.data) == self.max_len:
            raise QueueError("Overflow")
        
        if len(self.data) == 0 or rank >= self.data[len(self.data)-1][2]:
            self.data.append((val, time, rank))

        elif rank <= self.data[0][2]:
            self.data.insert(0, (val, time, rank))

        else:
            self.binsert(val, time, rank, 0, len(self.data))
        # print(self.data)
     
    def query(self, time=0, val=None, remove=False, return_rank=False) -> Optional[int]:
        """Queries a PIEO. Returns matching value and rank.
        Pops the PIEO if remove is True. Peeks otherwise.
        
        Takes in a time (default 0), and possibly a value. Uses the time for
        the eligibility predicate. If the value is passed in, returns the first
        eligible (ripe) value which matches the passed value parameter.
        """

        if len(self.data) == 0:
            raise QueueError("Underflow")
        
        if val == None:
            for x in range(len(self.data)):
                if self.ripe(self.data[x], time):
                    if return_rank:
                        return self.data.pop(x) if remove else self.data[x]
                    return self.data.pop(x)[0] if remove else self.data[x][0]
            raise QueueError("Underflow")
            
        for x in range(len(self.data)):
            #Find the first value that matches the query who is 'ripe'
            if self.data[x][0] == val and self.ripe(self.data[x], time):
                if return_rank:
                    return self.data.pop(x) if remove else self.data[x]
                return self.data.pop(x)[0] if remove else self.data[x][0]
        raise QueueError("Underflow")
    
    def pop(self, time=0, val=None, return_rank=False) -> Optional[int]:
        """Pops a PIEO. See query() for specifics."""

        return self.query(time, val, True, return_rank)

    def peek(self, time=0, val=None, return_rank=False) -> Optional[int]:
        """Peeks a PIEO. See query() for specifics."""

        return self.query(time, val, False, return_rank)

@dataclass
class PCQ:
    """A Programmable Calendar Queue (PCQ) data structure.
    Supports the operations `push`, `pop`, and `peek`.

    Elements are stored in buckets within which they are ordered by priority.
    Bucket priority is determined by the 'day pointer', which points to the current
    bucket with highest priority. 

    At initialization we take in a set of `(int, int)` tuple lists `data` which stores
    lists of values and their ranks, each representing a bucket.

    We store the highest rank of any element in the queue, such that we can upper bound the set of ranks.

    When asked to push:
    - We compute the bucket to push to as the rank of the new element multiplied by bucket width,
    wrapped around to be within the size of the queue.
    - We then insert said new element into its assigned bucket such that the rank order is preserved.

    When asked to pop:
    - If the length of `data` is 0, we raise an error .
    - Otherwise, we pop the lowest-rank element of the current day.
    - If, following our pop, the bucket is empty, we rotate to the next bucket.

    When asked to peek:
    We do the same thing as `pop`, except:
    - We peek into the PCQ instead of popping it - i.e. we don't remove any elements.

    We compactly represent these similar operations through `query`, which takes in an additional
    optional `remove` parameter (defaulted to False) to determine whether to pop or peek.
    """

    def __init__(self, max_len: int, num_buckets=200, width=10, initial_day=0):
        self.width = width
        self.day = initial_day
        self.max_len = max_len
        self.num_elements = 0
        self.data = []
        self.bucket_ranges = []
        self.highest_rank = 0
        for i in range(num_buckets):
            self.data.append(Pieo(200))
            self.bucket_ranges.append((i*width, (i+1)*width))

    
    def rotate(self) -> None:
        """Rotates a PCQ and changes the 'top' parameter of the previous bucket."""
        buckettop = self.bucket_ranges[self.day][1]
        self.bucket_ranges[self.day] = (buckettop, buckettop + self.width)
        self.day = (self.day + 1) % len(self.data)

    def push(self, val: int, time=0, rank=0) -> None:
        """Pushes a value with some rank/priority to a PCQ"""

        if self.num_elements == self.max_len:
            raise QueueError("Overflow")

        location = (rank * self.width) % len(self.data)

        try:
            self.data[location].push(val, time, rank)
            self.num_elements += 1
            if rank > self.highest_rank:
                self.highest_rank = rank
        except QueueError:
            raise QueueError("Overflow")

    
    def query(self, remove=False, time=0, val=None) -> Optional[int]:
        """Queries a PCQ."""

        if self.num_elements == 0:
            raise QueueError("Underflow")
    
        possible_values = []

        for bucket in self.data:
            try:
                peeked_val, peeked_time, peeked_rank = bucket.peek(time, val, True)
                possible_values.append((bucket, peeked_val, peeked_time, peeked_rank))
            except QueueError:
                continue
        if len(possible_values) > 0:
            possible_values.sort(key = lambda x : x[3])
            bucket, val, time, rank  = possible_values[0]
            if remove:
                return bucket.pop(time, val, False)
            else:
                return val
    
        raise QueueError("Underflow")
    
    def pop(self, time=0, val=None) -> Optional[int]:
        """Pops a PCQ. If we iterate through every bucket and can't find a value, raise underflow."""
        result = self.query(True, time, val)
        if result:
            self.num_elements -= 1
            return result
    
    def peek(self, time=0, val=None) -> Optional[int]:
        """Peeks a PCQ. If we iterate through every bucket and can't find a value, raise underflow."""
        return self.query(False, time, val)


def operate_queue(queue, max_cmds, commands, values, ranks=None, keepgoing=None, times=None):
    """Given the four lists:
    - One of commands, one of values, one of ranks, one of bounds:
    - Feed these into our queue, and return the answer memory.
    - Commands correspond to:
        0 : pop by predicate
        1 : peek by predicate
        2 : push
        3 : pop by value
        4 : peek by value
    """

    ans = []
    ranks = ranks or [0] * len(values)
    times = times or [0] * len(values)

    for cmd, val, rank, time in zip(commands, values, ranks, times):

        if cmd == 0: #Pop with time predicate
            try:
                ans.append(queue.pop(time))
            except QueueError:
                # print("Error popping with predicate")
                if keepgoing:
                    continue
                break
            
        elif cmd == 1: #Peek with time predicate
            try:
                ans.append(queue.peek(time))
            except QueueError:
                # print("Error peeking with predicate")
                if keepgoing:
                    continue
                break

        elif cmd == 2: #Push
            try:
                queue.push(val, time, rank)
            except QueueError:
                if keepgoing:
                    continue
                break

        elif cmd == 3: #Pop with value parameter
            try:
                ans.append(queue.pop(time, val))
            except QueueError:
                if keepgoing:
                    continue
                break

        elif cmd == 4: #Peek with value parameter
            try:
                ans.append(queue.peek(time, val))
            except QueueError:
                if keepgoing:
                    continue
                break
        
        else:
            raise CmdError("Unrecognized command.")

    # Pad the answer memory with zeroes until it is of length MAX_CMDS.
    ans += [0] * (max_cmds - len(ans))
    return ans