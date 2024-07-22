  # Queues

A queue is a standard data structure that maintains a set of elements in some total order.
A new element is added to the queue using the `enqueue` operation, which is also known as `push` or `insert` in some contexts.
Because of the total order, some element of the queue is the _most favorably ranked_ element.
We can read this element using the `peek` operation.
We can also remove this element from the queue using the `dequeue` operation, which is also known as `pop` or `remove` in some contexts.

We provision four types of queues in Calyx. The first three follow the same shared interface, while the fourth follows a slightly extended interface.
The frontend is implemented using the [Calyx builder library][builder], and the source code is heavily commented.

We first describe the shared interface and their associated testing harness and then detail the four types of queues.

## Shared Interface

All queues in Calyx expose the same interface.
- Input port `cmd`, a 2-bit integer.
Selects the operation to perform:
  - `0`: `pop`.
  - `1`: `peek`.
  - `2`: `push`.
- Input port `value`, a 32-bit integer.
The value to push.
- Register `ans`, a 32-bit integer that is passed to the queue by reference.
If `peek` or `pop` is selected, the queue writes the result to this register.
- Register `err`, a 1-bit integer that is passed to the queue by reference.
The queue raises this flag in case of overflow or underflow.

## Shared Testing Harness

Because they expose a common interface, we can test all queues using the same harness.


#### Data Generation
First, we have a Python module that generates randomized sequences of operations and values, and dumps the sequences out in a Calyx `.data` file.
It accepts as a command line argument the number of operations to generate.
It also takes a flag, `--no-err`, that generates a special sequence of operations that does not trigger any overflow or underflow errors.
If this flag is provided, the queue's length must also be provided.
The Python code is in [`queue_data_gen.py`][queue_data_gen.py].

#### Oracles
Next, we have a Python module _for each kind of queue_ that reads the `.data` file, simulates the queue in Python, and dumps the expected result out in a Calyx `.expect` file.
This Python code is aware of our [shared interface](#shared-interface).
The oracles are in [`fifo_oracle.py`][fifo_oracle.py], [`pifo_oracle.py`][pifo_oracle.py], [`pifo_tree_oracle.py`][pifo_tree_oracle.py], and [`binheap_oracle.py`][binheap_oracle.py].
They all appeal to pure-Python implementations of the queues, which are found in [`queues.py`][queues.py].
Each oracle also requires, as command line arguments, the number of operations being simulated and the queue's length.

#### Queue Call

The steps above lay out `.data` and `.expect` files for our Calyx code.
To actually pass a series of commands to a given eDSL queue implementation, we need to call the queue repeatedly with memories parsed from the `.data` file.
Unlike the above, which happens at the pure Python level, this needs to happen at the eDSL level.

This is exactly what [`queue_call.py`][queue_call.py] does.
It accepts as a Python-level argument a handle to the queue component that is to be tested.
It inserts a `main` component that reads the `.data` file and calls the queue component repeatedly.
If a command-line flag, `--keepgoing`, is provided, the `main` component will ignore any overflow or underflow errors raised by the queue component and will complete the entire sequence of operations.
If not, the `main` component will stop after the queue component first raises an error.

#### Putting It All Together

The testing harness can be executed, for all our queues, by running the shell script [`gen_queue_data_expect.sh`][gen_queue_data_expect.sh].
This generates the `.data` and `.expect` files for each queue.

Then, each queue can be tested using our `runt` setup.
The queue-generating Python files themselves expect command-line arguments (the number of operations and the optional `--keepgoing` flag) and you can see these being passed in the [relevant `runt` stanza][runt-queues].


## FIFO

The most basic queue is the _first in, first out_ (FIFO) queue: the most favorably ranked element is just the one that was added to the queue first.

Our queue frontend generates a simple FIFO in Calyx.
The source code is available [here][fifo.py].

Internally, our FIFO queue is implemented as a circular buffer.
One register marks the next element that would be popped, another marks the next cell into which an element would be pushed, and a third marks the number of elements in the queue.
The control logic is straightforward, and proceeds in three parallel arms based on the operation requested.

Using a circular buffer usually entails incrementing indices modulo the buffer size.
We use a trick to avoid this: we require the FIFO's length to be a power of two, say `2^k`, and we use adders of width `k` to increment the indices.
This means we can just naively increment the indices forever and the wrap-around behavior we want is automatically provided by overflow.

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

## Minimum Binary Heap

A minimum binary heap is another tree-shaped data structure where each node has at most two children.
However, unlike the queues discussed above, a heap exposes an extended interface:
in addition to the input ports and reference registers discussed above, a heap has an additional input `rank`.
The `push` operation now accepts both a `value` and the `rank` that the user wishes to associate with that value.
Consequently, a heap _orders_ its elements by `rank`, with the `pop` (resp. `peek`) operation set to remove (resp. read) the element with minimal rank.

To maintain this ordering efficiently, a heap stores `(rank, value)` pairs in each node and takes special care to maintain the following invariant:
> **Min-Heap Property**: for any given node `C`, if `P` is a parent of `C`, then the rank of `P` is less than or equal to the rank of `C`.

To `push` or `pop` an element is easy at the top level: write to or read from the correct node, and then massage the tree to restore the Min-Heap Property.
The `peek` operation is constant-time and `push` and `pop` are logarithmic in the size of the heap.

Our frontend allows for the creation of minimum binary heaps in Calyx; the source code is available in [`binheap.py`][binheap.py].

One quirk of any minimum binary heap is its ambiguous behavior in the case of rank ties.
More specifically, if the value `a` is pushed with some rank, and then later value `b` is pushed with the same rank, it's unclear which will be popped first.
Often, it's desirable to break such ties in FIFO order: that is we'd like a guarantee that `a` will be popped first.
A binary heap that provides this guarantee is called a _stable binary heap_, and our frontend provides a thin layer over our heap that enforces this property.

Our `stable_binheap` is a heap accepting 32-bit ranks and values.
It uses a counter `i` and instantiates, in turn, a binary heap that accepts 64-bit ranks and 32-bit values.
- To push a pair `(r, v)` into `stable_binheap`, we craft a new 64-bit rank that incorporates the counter `i` (specifically, we compute `r << 32 + i`), and we push `v` into our underlying binary heap with this new 64-bit rank. We also increment the counter `i`.
- To pop `stable_binheap`, we pop the underlying binary heap.
- To peek `stable_binheap`, we peek the underlying binary heap.

The source code is available in [`stable_binheap.py`][stable_binheap.py].

[builder]: ../builder/calyx-py.md
[fifo.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/test/correctness/queues/fifo.py
[pifo.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/test/correctness/queues/pifo.py
[binheap.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/test/correctness/queues/binheap/binheap.py
[stable_binheap.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/test/correctness/queues/binheap/stable_binheap.py
[pifo_tree.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/test/correctness/queues/pifo_tree.py
[sivaraman16]: https://dl.acm.org/doi/10.1145/2934872.2934899
[mohan23]: https://dl.acm.org/doi/10.1145/3622845
[queue_data_gen.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/calyx/queue_data_gen.py
[queues.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/calyx/queues.py
[fifo_oracle.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/calyx/fifo_oracle.py
[pifo_oracle.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/calyx/pifo_oracle.py
[pifo_tree_oracle.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/calyx/pifo_tree_oracle.py
[binheap_oracle.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/calyx/binheap_oracle.py
[gen_queue_data_expect.sh]: https://github.com/calyxir/calyx/blob/main/calyx-py/calyx/gen_queue_data_expect.sh
[queue_call.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/calyx/queue_call.py
[runt-queues]: https://github.com/calyxir/calyx/blob/a4c2442675d3419be6d2f5cf912aa3f804b3c4ab/runt.toml#L131-L144
