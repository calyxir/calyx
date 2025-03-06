from dataclasses import dataclass, field
from typing import Any, Optional
import heapq

ERR_CODE = 2**32 - 1
PUSH_CODE = 2**32 - 2


class QueueError(Exception):
    """An error that occurs on popping from an empty queue or pushing to a full one"""


class CmdError(Exception):
    """An error that occurs if an undefined command is passed"""


@dataclass
class Fifo:
    """A FIFO data structure.
    Supports the operations `push` and `pop``.
    Inherent to the queue is its `max_len`,
    which is given at initialization and cannot be exceeded.
    """

    def __init__(self, max_len: int):
        self.data = []
        self.max_len = max_len

    def push(self, val: int, *_) -> None:
        """Pushes `val` to the FIFO."""
        if len(self.data) == self.max_len:
            raise QueueError("Cannot push to full FIFO.")

        self.data.append(val)

    def pop(self, *_) -> int:
        """Pops the FIFO."""
        if len(self.data) == 0:
            raise QueueError("Cannot pop from empty FIFO.")

        return self.data.pop(0)

    def __len__(self) -> int:
        return len(self.data)

    def __str__(self) -> str:
        return str(self.data)


@dataclass(order=True)
class RankValue:
    priority: int
    value: Any = field(compare=False)


@dataclass
class NWCSimple:
    """A simple test oracle structure for non-work-conserving queues.

    This serves the same purpose as PIEOs and Calendar Queues
    (see Python implementation below), representing an abstract implementation
    for any non-work-conserving, time-dependent structure.

    In our implementation, we support time-based 'ripeness' predicates, which
    check that an element's encoded 'readiness time' is earlier than the
    specified current time.

    The term 'ripe', as defined above, takes in an element
    (which has some encoded readiness time), and a specified 'current time'.
    It checks that the element's readiness time is <= the current time.

    Supports the operations `push` and `pop`.

    Stores elements in the form of a heap (using the `heapq` library).

    At initialization, we take a `max_len` value to store the maximum possible
    length of a queue.

    When asked to push:
    - If the queue is at length `max_len`, we raise an error.
    - Otherwise, we insert the element into the PIEO such that the rank order
    stays increasing.
    - To avoid ties between ranks, we left-shift the rank and then add either
    a custom buffer, or an internal element count tracker.

    When asked to pop:
    - If the length of `data` is 0, we raise an error .
    - We can either pop based on value or based on eligibility.
    - This implementation supports the most common readiness predicate -
        whether an element is 'ripe' or time-ready
        (the inputted time is >= the element's specified readiness time).
    - If a value is passed in, we pop the first (lowest-rank) instance of that
    value which is 'ripe'.
    - If no value is passed in but a time is, we pop the first (lowest-rank)
    value that passes the predicate.
    """

    def __init__(self, max_len: int):
        self.data = []
        self.max_len = max_len
        self.insertion_count = 0

    def push(self, val: int, rank: int = 0, time: int = 0) -> None:
        if len(self.data) >= self.max_len:
            raise QueueError("Cannot push to full queue.")

        # Left shift the rank by 32 and add in the insertion count.
        # This breaks rank ties in insertion order: that is, if two elements are
        # inserted with the same rank, the element inserted first gets popped first.

        heapq.heappush(
            self.data, RankValue(((rank << 32) + self.insertion_count), (val, time))
        )
        self.insertion_count += 1

    def is_ripe(self, time: int) -> bool:
        return self.data[0].value[1] <= time

    def query(self, time: int = 0, val: Optional[int] = None) -> int:
        if len(self.data) == 0:
            raise QueueError("Cannot pop from empty queue.")

        # Cache popped values from heap while searching for the first eligible one
        temp = []

        while len(self.data) > 0:
            # Check for eligibility
            if self.is_ripe(time) and (val is None or self.data[0].value[0] == val):
                # If eligible, we pop the element and push all cached elements
                # back into the heap
                result = heapq.heappop(self.data)

                for elem in temp:
                    heapq.heappush(self.data, elem)

                return result.value[0]

            # After each iteration, pop the current element so we can scan down the heap
            temp.append(heapq.heappop(self.data))

        # If no eligible elements are found, repopulate the data heap with
        # cached elements
        for elem in temp:
            heapq.heappush(self.data, elem)

        raise QueueError("Cannot pop from empty queue.")

    def pop(self, time: int = 0, val: Optional[int] = None) -> int:
        return self.query(time, val)


@dataclass
class Pieo:
    """A PIEO data structure.
    PIEOs function as generalized PIFOs, but popping and pushing supports the
    'extract-out' idea rather than exclusively a 'first-out' operation.
    Elements can either be extracted by value
    (pass in a value and obtain the lowest-rank element matching it), or by an
    eligibility predicate (find the lowest-rank element matching the predicate).

    In our implementation, we support time-based 'ripeness' predicates,
    which check that an element's encoded 'readiness time' is earlier than the
    specified current time.

    The term 'ripe', as defined above, takes in an element
    (which has some encoded readiness time),
    and a specified 'current time'. It checks that the element's readiness time
    is <= the current time.

    For more info, consult https://dl.acm.org/doi/pdf/10.1145/3341302.3342090.

    Supports the operations `push` and `pop`.

    Stores elements ordered increasingly by a totally ordered `rank` attribute (for
    simplicity, our implementation is just using integers).

    At initialization, we take a `max_len` value to store the maximum possible
    length of a queue.

    When asked to push:
    - If the PIEO is at length `max_len`, we raise an error.
    - Otherwise, we insert the element into the PIEO such that the rank order
    stays increasing.
    - To avoid ties between ranks, we left-shift the rank and then add either a
    custom buffer,
        or an internal element count tracker.

    When asked to pop:
    - If the length of `data` is 0, we raise an error .

    - We can either pop based on value or based on eligibility.
    - This implementation supports the most common readiness predicate - whether
      an element is 'ripe', or time-ready
      (the inputted time is >= the element's specified readiness time).

    - If a value is passed in, we pop the first (lowest-rank) instance of that
      value which is 'ripe'.
    - If no value is passed in but a time is,
        we pop the first (lowest-rank) value that passes the predicate.
    """

    def __init__(self, max_len: int):
        """Initialize structure."""
        self.max_len = max_len
        self.data = []
        self.insertion_count = 0

    def is_ripe(self, element, time):
        """Check that an element is 'ripe' - i.e. its ready time has passed"""
        return element["time"] <= time

    def binsert(self, val, time, rank, left, r):
        """Inserts element into list such that rank ordering is preserved
        Uses variant of binary search algorithm.
        """
        if left == r:
            return self.data.insert(left, {"val": val, "time": time, "rank": rank})

        mid = (left + r) // 2

        if rank == self.data[mid]["rank"]:
            return self.data.insert(mid, {"val": val, "time": time, "rank": rank})

        if rank > self.data[mid]["rank"]:
            return self.binsert(val, time, rank, mid + 1, r)

        if rank < self.data[mid]["rank"]:
            return self.binsert(val, time, rank, left, mid)

    def push(self, val, rank=0, time=0, insertion_count=None) -> None:
        """Pushes to a PIEO.
        Inserts element such that rank ordering is preserved
        """

        # Breaks ties and maintains FIFO order (can pass either custom insertion
        # order or use PIEO internal one).
        # Left-shifts the rank 32 bits, before adding either a passed in
        # `insertion_count` parameter or the internal one.
        rank = (rank << 32) + (insertion_count or self.insertion_count)

        # If there is no room left in the queue, raise an Overflow error
        if len(self.data) == self.max_len:
            raise QueueError("Overflow")

        # If there are no elements in the queue, or the latest rank is higher
        # than all others, append to the end
        if len(self.data) == 0 or rank >= self.data[len(self.data) - 1]["rank"]:
            self.data.append({"val": val, "time": time, "rank": rank})

        # If the latest rank is lower than all others, insert to the front
        elif rank <= self.data[0]["rank"]:
            self.data.insert(0, {"val": val, "time": time, "rank": rank})

        # Otherwise, use the log-time insertion function
        else:
            self.binsert(val, time, rank, 0, len(self.data))

        self.insertion_count += 1

    def query(self, time=0, val=None, remove=False, return_rank=False) -> Optional[int]:
        """Queries a PIEO. Returns matching value and rank.
        Pops the PIEO if remove is True. Peeks otherwise.

        Takes in a time (default 0), and possibly a value. Uses the time for
        the eligibility predicate. If the value is passed in, returns the first
        eligible (ripe) value which matches the passed value parameter.
        """

        if len(self.data) == 0:
            raise QueueError("Underflow")

        # If there is only a time predicate
        if val is None:
            # Iterate until we find the first 'ripe' (time-ready) element
            for x in range(len(self.data)):
                if self.is_ripe(self.data[x], time):
                    if return_rank:
                        return self.data.pop(x)
                    return self.data.pop(x)["val"]

            # No ripe elements
            raise QueueError("Underflow")

        # Otherwise, the first element that matches the queried value & is 'ripe'
        for x in range(len(self.data)):
            if self.data[x]["val"] == val and self.is_ripe(self.data[x], time):
                if return_rank:
                    return self.data.pop(x)
                return self.data.pop(x)["val"]

        # No ripe elements matching value
        raise QueueError("Underflow")

    def pop(self, time=0, val=None, return_rank=False) -> Optional[int]:
        """Pops a PIEO. See query() for specifics."""

        return self.query(time, val, True, return_rank)


@dataclass
class PCQ:
    """A Programmable Calendar Queue (PCQ) data structure.
    Supports the operations `push` and `pop` by time predicate and value.

    See the papers https://www.usenix.org/system/files/nsdi20-paper-sharma.pdf and
    https://dl.acm.org/doi/pdf/10.1145/63039.63045 for details.

    Elements are stored in buckets within which they are ordered by priority. For our implementation,
    each bucket takes on the form of a PIEO. Bucket priority is determined by the 'day pointer',
    which points to the current bucket with highest priority.

    At initialization we initialize `data` to be a list of empty buckets, each of which
    has the form of a PIEO with max length 16.

    We store the number of elements that have already been inserted into the queue, initialized at 0.
    With each iteration, we increment this.

    When inserting elements into the queue, we pass in this parameter along with the specified rank,
    so that it is factored into a modified rank calculation that removes all ties between ranks.
    (See PIEO documentation for details of this modified rank calculation.)

    When asked to push:
    - We compute the bucket to push to as the rank of the new element multiplied by bucket width,
    wrapped around to be within the size of the queue.
    - We then insert said new element into its assigned bucket such that the rank order is preserved.

    When asked to pop:
    - If the length of `data` is 0, we raise an error .
    - Otherwise, we pop the lowest-rank element in the queue.
    - If, following our pop, the bucket is empty, we rotate to the next bucket.

    """

    def __init__(self, max_len: int, num_buckets=200, width=10, initial_day=0):
        self.width = width
        self.day = initial_day
        self.max_len = max_len
        self.num_elements = 0
        self.data = []
        self.bucket_ranges = []
        self.insertion_count = 0
        for i in range(num_buckets):
            self.data.append(Pieo(16))
            self.bucket_ranges.append((i * width, (i + 1) * width))

    def rotate(self) -> None:
        """Rotates a PCQ and changes the 'top' parameter of the previous bucket."""
        buckettop = self.bucket_ranges[self.day][1]
        self.bucket_ranges[self.day] = (buckettop, buckettop + self.width)
        self.day = (self.day + 1) % len(self.data)

    def push(self, val: int, rank=0, time=0) -> None:
        """Pushes a value with some rank/priority to a PCQ"""

        if self.num_elements == self.max_len:
            raise QueueError("Overflow")

        location = (rank * self.width) % len(self.data)

        try:
            self.data[location].push(val, rank, time, self.insertion_count)
            self.num_elements += 1
            self.insertion_count += 1
        except QueueError:
            raise QueueError("Overflow")

    def query(self, time=0, val=None) -> Optional[int]:
        """Queries a PCQ."""

        if self.num_elements == 0:
            raise QueueError("Underflow")

        possible_values = []

        for bucket in self.data:
            try:
                peeked = bucket.peek(time, val, True)
                possible_values.append((bucket, peeked))
            except QueueError:
                continue
        if len(possible_values) > 0:
            possible_values.sort(key=lambda x: x[1]["rank"])
            bucket, element = possible_values[0]
            time, val = element["time"], element["val"]

            result = bucket.pop(time, val, False)
            self.num_elements -= 1
            if len(bucket.data) == 0:
                self.rotate()
            return result

        raise QueueError(str(self.data) + "Underflow")

    def pop(self, time=0, val=None) -> Optional[int]:
        """Pops a PCQ. If we iterate through every bucket and can't find a value,
        raise underflow."""

        return self.query(time, val)


@dataclass
class Binheap:
    """A minimum Binary Heap data structure.
    Supports the operations `push` and `pop`.
    """

    def __init__(self, max_len: int):
        self.heap = []
        self.len = 0
        self.counter = 0
        self.max_len = max_len

    def push(self, val: int, rank: int, *_) -> None:
        """Pushes `(rnk, val)` to the Binary Heap."""
        if self.len == self.max_len:
            raise QueueError("Cannot push to full Binary Heap.")

        self.counter += 1
        self.len += 1
        heapq.heappush(self.heap, RankValue((rank << 32) + self.counter, val))

    def pop(self, *_) -> int:
        """Pops the Binary Heap."""
        if self.len == 0:
            raise QueueError("Cannot pop from empty Binary Heap.")

        self.len -= 1
        return heapq.heappop(self.heap).value

    def __len__(self) -> int:
        return self.len


@dataclass
class RRPifo:
    """
    This is a version of a PIFO generalized to `n` flows, with a work conserving
    round robin policy. If a flow is silent when it is its turn, that flow
    simply skips its turn and the next flow is offered service.

    Supports the operations `push` and `pop`.

    It takes in a list `boundaries` that must be of length `n`, using which the
    client can divide the incoming traffic into `n` flows.
    For example, if n = 3 and the client passes boundaries [133, 266, 400],
    packets will be divided into three flows: [0, 133], [134, 266], [267, 400].
    The argument `subqueues` is a list of subqueues, which can be any type of
    queue from this module.

    - At push, we check the `boundaries` list to determine which flow to push to.
    Take the boundaries example given earlier, [133, 266, 400].
    If we push the value 89, it will end up in flow 0 because 89 <= 133,
    and 305 would end up in flow 2 since 266 <= 305 <= 400.
    - Pop first tries to pop from `hot`. If this succeeds, great. If it fails,
    it increments `hot` and therefore continues to check all other flows
    in round robin fashion.
    """

    def __init__(self, n: int, boundaries: list, subqueues: list, max_len: int):
        self.hot = 0
        self.n = n
        self.pifo_len = 0
        self.boundaries = boundaries
        self.data = subqueues
        self.max_len = max_len

        # Can't initialize a PIFO that is too long.
        assert self.pifo_len <= self.max_len
        # Need one boundary per flow.
        assert len(self.boundaries) == self.n
        # Need one subqueue per flow.
        assert len(self.data) == self.n
        # `boundaries` must be strictly increasing.
        assert sorted(set(self.boundaries)) == self.boundaries

    def push(self, val: int, *_) -> None:
        """Pushes `val` to the PIFO."""
        if self.pifo_len == self.max_len:
            raise QueueError("Cannot push to full PIFO.")

        for subqueue, boundary in zip(self.data, self.boundaries):
            if val <= boundary:
                subqueue.push(val)
                self.pifo_len += 1
                return

        raise QueueError(f"Cannot push value > {max(self.boundaries)}.")

    def increment_hot(self) -> None:
        """Increments `hot`, taking into account wraparound."""
        self.hot = (self.hot + 1) % self.n

    def pop(self, *_) -> int:
        """Pops the PIFO by popping some internal subqueue.
        Updates `hot` to be one more than the index of the internal subqueue that
        we popped.
        """
        if self.pifo_len == 0:
            raise QueueError("Cannot pop from empty PIFO.")

        while True:
            try:
                val = self.data[self.hot].pop()
                self.increment_hot()
                self.pifo_len -= 1
                return val
            except QueueError:
                self.increment_hot()

    def __len__(self) -> int:
        return self.pifo_len


@dataclass
class StrictPifo:
    """
    This is a version of a PIFO generalized to `n` flows, with a strict policy.
    Flows have a strict order of priority, which determines popping
    order. If the highest priority flow is silent when it is its turn, that flow
    simply skips its turn and the next flow is offered service. If a higher
    priority flow get pushed to in the interim, the next call to pop will
    return from that flow.

    Supports the operations `push`, and `pop`.
    It takes in a list `boundaries` that must be of length `n`, using which the
    client can divide the incoming traffic into `n` flows.
    For example, if n = 3 and the client passes boundaries [133, 266, 400],
    packets will be divided into three flows: [0, 133], [134, 266], [267, 400].

    It takes a list `order` that must be of length `n`, which specifies the order
    of priority of the flows. For example, if n = 3 and the client passes order
    [1, 2, 0], flow 1 (packets in range [134, 266]) is first priority, flow 2
    (packets in range [267, 400]) is second priority, and flow 0 (packets in range
    [0, 133]) is last priority. The argument `subqueues` is a list of subqueues,
    which can be any type of queue from this module.

    - At push, we check the `boundaries` list to determine which flow to push to.
    Take the boundaries example given earlier, [133, 266, 400].
    If we push the value 89, it will end up in flow 0 because 89 <= 133,
    and 305 would end up in flow 2 since 266 <= 305 <= 400.
    - Pop first tries to pop from `order[0]`. If this succeeds, great. If it fails,
    it tries `order[1]`, etc.
    """

    def __init__(
        self, n: int, boundaries: list, order: list, subqueues: list, max_len: int
    ):
        self.order = order
        self.priority = 0
        self.n = n
        self.pifo_len = 0
        self.boundaries = boundaries
        self.data = subqueues
        self.max_len = max_len

        # Can't initialize a PIFO that is too long.
        assert self.pifo_len <= self.max_len
        # Need one boundary per flow.
        assert len(self.boundaries) == self.n
        # Need one subqueue per flow.
        assert len(self.data) == self.n
        # `boundaries` must be strictly increasing.
        assert sorted(set(self.boundaries)) == self.boundaries

    def push(self, val: int, *_) -> None:
        """Works the same as in RRQueue. Pushes `val` to the PIFO."""
        if self.pifo_len == self.max_len:
            raise QueueError("Cannot push to full PIFO.")

        for b in range(self.n):
            if val <= self.boundaries[b]:
                idx = self.order.index(b)
                self.data[idx].push(val)
                self.pifo_len += 1
                return

        raise QueueError(f"Cannot push value > {max(self.boundaries)}.")

    def next_priority(self) -> None:
        """Increments priority, taking into account wrap around."""
        self.priority = (self.priority + 1) % self.n

    def pop(self, *_) -> int:
        """Pops the PIFO."""
        if self.pifo_len == 0:
            raise QueueError("Cannot pop from empty PIFO.")

        original_priority = self.priority

        while True:
            try:
                val = self.data[self.priority].pop()
                self.priority = original_priority
                self.pifo_len -= 1
                return val
            except QueueError:
                self.next_priority()

    def __len__(self) -> int:
        return self.pifo_len


def operate_queue(
    queue, max_cmds, commands, values, ranks=None, times=None, keepgoing=None
):
    """Given the four lists
    (One of commands, one of values, one of ranks, one of times):
    - Note that `commands` and `values` are required,
      while `ranks` and `times` are optional lists depending on the queue type.
    - Feed these into our queue, and return the answer memory.
    - Commands correspond to:
        0 : pop (for non-work-conserving queues, pop by predicate)
        1 : push
    """

    ans = []
    ranks = ranks or [0] * len(values)
    times = times or [0] * len(values)

    for cmd, val, rank, time in zip(commands, values, ranks, times):
        if cmd == 0:  # Pop (with possible time predicate)
            try:
                ans.append(queue.pop(time))
            except QueueError:
                ans.append(ERR_CODE)
                if keepgoing:
                    continue
                break

        elif cmd == 1:  # Push
            try:
                queue.push(val, rank, time)
                ans.append(PUSH_CODE)
            except QueueError:
                ans.append(ERR_CODE)
                if keepgoing:
                    continue
                break

        else:
            raise CmdError("Unrecognized command.")

    # Pad the answer memory with zeroes until it is of length MAX_CMDS.
    ans += [0] * (max_cmds - len(ans))
    return ans
