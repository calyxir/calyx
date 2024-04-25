  # Queues

A queue is a standard data structure that maintains a set of elements in some total order.
A new element is added to the queue using the `enqueue` operation, which is also known as `push` or `insert` in some contexts.
Because of the total order, some element of the queue is the _most favorably ranked_ element.
We can read this element using the `peek` operation.
We can also remove this element from the queue using the `dequeue` operation, which is also known as `pop` or `remove` in some contexts.

We provision three types of queues in Calyx, which follow the same interface for ease of use.
The frontend is implemented using the [Calyx builder library][builder], and the source code is heavily commented.

We first describe the shared interface and then detail the three types of queues.

## Shared Interface

All queues in Calyx expose the same interface.
- Input port `cmd`, a 2-bit integer.
Selects the operation to perform:
  - `0`: `pop`.
  - `1`: `peek`.
  - `2`: `push`.
- Input port `value`, a 32-bit integer
The value to push.
- Register `ans`, a 32-bit integer that is passed to the queue by reference.
If `peek` or `pop` is selected, the queue writes the result to this register.
- Register `err`, a 1-bit integer that is passed to the queue by reference.
The queue raises this flag in case of overflow or underflow.


## FIFO

The most basic queue is the _first in, first out_ (FIFO) queue: the most favorably ranked element is just the one that was added to the queue first.

Our queue frontend generates a simple FIFO in Calyx.
The source code is available [here][fifo.py].

Internally, our FIFO queue is implemented as a circular buffer.
One register marks the next element that would be popped, another marks the next cell into which an element would be pushed, and a third marks the number of elements in the queue.
The control logic is straightforward, and proceeds in three parallel arms based on the operation requested.

## PIFO

A more complex instance is the priority queue.
At time of enqueue, an element is associated with a priority.
The most favorably ranked element is the one with the highest priority.
Two elements may be pushed with the same priority.
A priority queue that is additionally defined to break such ties in FIFO order is called a _push in, first out_ (PIFO) queue.

Our queue frontend generates a simple PIFO in Calyx; the source code is available [here][pifo.py].

Curiously, our PIFO has a ranking policy "baked in": it partitions incoming elements into two classes, and tries to emit elements from those two classes in a round-robin fashion.
The PIFO operates in a work-conserving manner: if there are no elements from one class and there are elements from the other, we emit an element from the latter class even if it is not its turn.

Internally, our PIFO maintains two sub-queues, one for each class.
It also has a boundary value, which informs its partition policy: elements less than the boundary go to the first class, and other elements go to the second class.
Control logic for pushing a new element is straightforward.
The control logic for peeking and popping is more subtle because this is where the round-robin policy is enforced.
A register tracks the class that we wish to emit from next.
The register starts arbitrarily, and is updated after each successful emission from the desired class.
It is left unchanged in the case when the desired class is empty and an element of the other class is emitted in the interest of work conservation.

## PIFO Tree

A PIFO tree is a tree-shaped data structure in which each node is associated with a PIFO.
The PIFO tree stores elements in its leaf PIFOs and scheduling metadata in its internal nodes.

Pushing a new element into a PIFO tree involves putting the element into a leaf PIFO and putting additional metadata into internal nodes from the root to the leaf.
A variety of scheduling policies can be realized by manipulating the various priorities with which this data is inserted into each PIFO.
Popping the most favorably ranked element from a PIFO tree is relatively straightforward: popping the root PIFO tells us which child PIFO to pop from next, and we recurse until we reach a leaf PIFO.
We refer interested readers to [this][sivaraman16] research paper for more details on PIFO trees.

Our frontend allows for the creation of PIFO trees of any height, but with two restrictions:
- The tree must be binary-branching.
- The scheduling policy at each internal node must be _round-robin_.

See the [source code][pifo_tree.py] for an example where we create a PIFO tree of height 2.
Specifically, the example implements the PIFO tree described in §2 of [this][mohan23] research paper.

Internally, our PIFO tree is implemented by leveraging the PIFO frontend.
The PIFO frontend seeks to orchestrate two queues, which in the simple case will just be two FIFOs.
However, it is easy to generalize those two queues: instead of being FIFOs, they can be PIFOs or even PIFO trees.


[builder]: ../builder/calyx-py.md
[fifo.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/test/correctness/queues/fifo.py
[pifo.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/test/correctness/queues/pifo.py
[pifo_tree.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/test/correctness/queues/pifo_tree.py
[sivaraman16]: https://dl.acm.org/doi/10.1145/2934872.2934899
[mohan23]: https://dl.acm.org/doi/10.1145/3622845