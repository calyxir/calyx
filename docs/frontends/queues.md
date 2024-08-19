  # Queues

A queue is a standard data structure that maintains a set of elements in some total order.
A new element is added to the queue using the `enqueue` operation, which is also known as `push` or `insert` in some contexts.
Because of the total order, some element of the queue is the _most favorably ranked_ element.
We can remove this element from the queue using the `dequeue` operation, which is also known as `pop` or `remove` in some contexts.

We provision four types of queues in Calyx. The first three follow the same shared interface, while the fourth follows a slightly extended interface.
The frontend is implemented using the [Calyx builder library][builder], and the source code is heavily commented.

We first describe the shared interface and their associated testing harness and then detail the four types of queues.

## Shared Interface

All queues in Calyx expose the same interface.
- Input port `cmd`, a 2-bit integer.
Selects the operation to perform:
  - `0`: `pop`.
  - `1`: `push`.
- Input port `value`, a 32-bit integer.
The value to push.
- Register `ans`, a 32-bit integer that is passed to the queue by reference.
If `pop` is selected, the queue writes the result to this register.
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

## Specialized PIFOs

A more complex instance is the priority queue.
At time of enqueue, an element is associated with a priority.
The most favorably ranked element is the one with the highest priority.
Two elements may be pushed with the same priority; a priority queue that is additionally defined to break such ties in FIFO order is called a _push in, first out_ (PIFO) queue.

We provide PIFOs in the general sense (i.e., queues that accept `(item, rank)` pairs and enqueue based on `rank`) [shortly](#minimum-binary-heap). For now, let's focus on specialized PIFOs that have a policy "baked in" to the queue itself.

We have two types of specialized PIFOs - Round Robin and Strict - that implement policies which determine which flow to pop from next. These PIFOs are parameterized over the number of flows, `n`, that they can arbitrate between.

### Round-Robin Queues

Round robin queues are PIFOs generalized to `n` flows that operate in a work
conserving round-robin fashion. That is, if a flow is silent when it is its turn, that flow
simply skips its turn and the next flow is offered service.

Internally, it operates `n` subqueues.
It takes in a list `boundaries` that must be of length `n`, using which the
client can divide the incoming traffic into `n` flows.
For example, if `n = 3` and the client passes boundaries `[133, 266, 400]`,
packets will be divided into three flows according to the intervals: `[0, 133]`, `[134, 266]`, `[267, 400]`.

- At `push`, we check the `boundaries` list to determine which flow to push to.
Take the boundaries example given earlier, `[133, 266, 400]`.
If we push the value `89`, it will, under the hood, be pushed into subqueue 0 becuase `0 <= 89 <= 133`,
and `305` will be pushed into subqueue 2 since `266 <= 305 <= 400`.
- The program maintains a `hot` pointer that starts off at 0, meaning the next subqueue to pop from is queue 0.
At `pop` we first try to pop from `hot`. If this succeeds, great. If it fails,
we increment `hot` and therefore continue to check all other flows
in round robin fashion.

The source code is available in [`gen_strict_or_rr.py`][gen_strict_or_rr.py], which takes as arguments `n`, `boundaries`, and handles to the subqueues it must administer. It also takes a boolean parameter `round_robin`, which, if `true`, results in the generation of a round-robin queue.


### Strict Queues

Strict queues support `n` flows as well, but instead, flows have a strict order of priority and this which determines popping
order. That is, the second-highest priority subqueue will only be allowed to pop if the highest priority subqueue is empty.
If the higher-priority flow get pushed to in the interim, the next call to `pop` will again try to pop from the highest priority flow.

Like round-robin queues, it takes in a list `boundaries` that must be of length `n`, which divide the incoming traffic into `n` flows.
For example, if `n = 3` and the client passes boundaries `[133, 266, 400]`,
packets will be divided into three flows according to the intervals: `[0, 133]`, `[134, 266]`, `[267, 400]`.

It takes a list `order` that must be of length `n`, which specifies the order
of priority of the flows. For example, if `n = 3` and the client passes order
`[1, 2, 0]`, then flow 1 (packets in range `[134, 266]`) has first priority, flow 2
(packets in range `[267, 400]`) has second priority, and flow 0 (packets in range
`[0, 133]`) has last priority.

- At push, we check the `boundaries` list to determine which flow to push to.
Take the boundaries example given earlier, `[133, 266, 400]`.
If we push the value `89`, it will, under the hood, be pushed into subqueue 0 becuase `0 <= 89 <= 133`,
and `305` will be pushed into subqueue 2 since `266 <= 305 <= 400`.
- Pop first tries to pop from `order[0]`. If this succeeds, great. If it fails, it tries `order[1]`, and so on.

The source code is available in [`gen_strict_or_rr.py`][gen_strict_or_rr.py], which takes as arguments `n`, `boundaries`, `order`, and handles to the subqueues it must administer. It also takes a boolean parameter `round_robin`, which, if `false`, results in the generation of a strict queue.

## PIFO Tree

A PIFO tree is a tree-shaped data structure in which each node is associated with a PIFO.
The PIFO tree stores elements in its leaf PIFOs and scheduling metadata in its internal nodes.

Pushing a new element into a PIFO tree involves putting the element into a leaf PIFO and putting additional metadata into internal nodes from the root to the leaf.
A variety of scheduling policies can be realized by manipulating the various priorities with which this data is inserted into each PIFO.
Popping the most favorably ranked element from a PIFO tree is relatively straightforward: popping the root PIFO tells us which child PIFO to pop from next, and we recurse until we reach a leaf PIFO.
We refer interested readers to [this][sivaraman16] research paper for more details on PIFO trees.

Our frontend allows for the creation of PIFO trees of any height, number of children, and with
the scheduling policy at each internal node being _round-robin_ or _strict_.

See the [source code][pifo_tree.py] for an example where we create a PIFO tree of height 2.
Specifically, the example implements the PIFO tree described in §2 of [this][mohan23] research paper.

Internally, our PIFO tree is implemented by leveraging the PIFO frontend.
The PIFO frontend seeks to orchestrate two queues, which in the simple case will just be two FIFOs.
However, it is easy to generalize those two queues: instead of being FIFOs, they can be PIFOs or even PIFO trees.

We see a more complex example of a PIFO tree in [`complex_tree.py`] [complex_tree.py]. This tree does round robin between three children, two of which are strict queues and the other is a round robin queue. This tree has a height of 3. The overall structure is `rr(strict(A, B, C), rr(D, E, F), strict(G, H))`.

## Minimum Binary Heap

A minimum binary heap is another tree-shaped data structure where each node has at most two children.
However, unlike the queues discussed above, a heap exposes an extended interface:
in addition to the input ports and reference registers discussed above, a heap has an additional input `rank`.
The `push` operation now accepts both a `value` and the `rank` that the user wishes to associate with that value.
Consequently, a heap _orders_ its elements by `rank`, with the `pop` operation set to remove the element with minimal rank.

To maintain this ordering efficiently, a heap stores `(rank, value)` pairs in each node and takes special care to maintain the following invariant:
> **Min-Heap Property**: for any given node `C`, if `P` is a parent of `C`, then the rank of `P` is less than or equal to the rank of `C`.

To `push` or `pop` an element is easy at the top level: write to or read from the correct node, and then massage the tree to restore the Min-Heap Property.
The `push` and `pop` operations are logarithmic in the size of the heap.

Our frontend allows for the creation of minimum binary heaps in Calyx; the source code is available in [`binheap.py`][binheap.py].

One quirk of any minimum binary heap is its ambiguous behavior in the case of rank ties.
More specifically, if the value `a` is pushed with some rank, and then later value `b` is pushed with the same rank, it's unclear which will be popped first.
Often, it's desirable to break such ties in FIFO order: that is we'd like a guarantee that `a` will be popped first.
A binary heap that provides this guarantee is called a _stable binary heap_, and our frontend provides a thin layer over our heap that enforces this property.

Our `stable_binheap` is a heap accepting 32-bit ranks and values.
It uses a counter `i` and instantiates, in turn, a binary heap that accepts 64-bit ranks and 32-bit values.
- To push a pair `(r, v)` into `stable_binheap`, we craft a new 64-bit rank that incorporates the counter `i` (specifically, we compute `r << 32 + i`), and we push `v` into our underlying binary heap with this new 64-bit rank. We also increment the counter `i`.
- To pop `stable_binheap`, we pop the underlying binary heap.

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
[gen_strict_or_rr.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/test/correctness/queues/strict_and_rr_queues/gen_strict_or_rr.py
[complex_tree.py]: https://github.com/calyxir/calyx/blob/main/calyx-py/test/correctness/queues/complex_tree.py
