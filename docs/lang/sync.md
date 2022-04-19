# Experimental: Synchronization

Calyx's default semantics [do not admit][par-undef] any predictable form of language-level
synchronization in presence of parallelism.
We're currently experimenting with a suite of new primitives that add synchronization to the
language.

## `std_sync_reg`

The `std_sync_reg` primitive defined by `primitives/sync.futil` provides a synchronizing
register that acts as an [M-structure][m-struct] which provides the following interface:
On the reader side:
1. If the register is "empty", block the read till the register is written into.
2. If the register is "full", provide the value to the reader, provide a `done` signal, and mark it as "empty".

On the writer side:
1. If the register is "empty", write the value in the register, mark it as "full", and provide a `done` signal.
2. If the register is "full", block the write till the register is read from.

One way to think of this interface is as a size-1 concurrent FIFO.

## Using `std_sync_reg`

> The [following example][ex] is a part of the Calyx compiler test suite and can be
> executed using:
>
>       runt -i tests/correctness/sync

The synchronizing register interface is non-standard: it provides two go signals and
two done signals to initiate parallel reads and writes.

```
{{#include ../../primitives/sync.futil:sync_interface}}
```

The signal `read_en` is used by a program to initiate a read operation while
the `write_en` signal initiates a write operation.
We need to explicitly initiate a read operation because reading a value marks
the register as "empty" which causes any future reads to block.

Similarly, the output interface specifies the `read_done` and `write_done` signals
which the user program needs to read to know when the operations are completed.
The `read_done` signal is similar to a `valid` signal while the `write_done` is
similar to a `write_done` signal.

The following group initiates a write operation into the synchronizing register `imm`
from the memory `in`:
```
{{#include ../../tests/correctness/sync.futil:write}}
```
The group waits for the `imm.write_done` signal to be high before continuing
execution.
If the synchronizing register was "full" in this cycle, the execution would
stall and cause the group to take another cycle.

The following group initiates a read the synchronizing register `imm` and saves
the value into the `out` memory:
```
{{#include ../../tests/correctness/sync.futil:read}}
```
The group waits till the `imm.read_done` signal is high to write the value into
the memory.
Note that in case the register is empty, `imm.read_done` will be low and cause
the group to another cycle.

Finally, we can describe the control program as:
```
{{#include ../../tests/correctness/sync.futil:control}}
```
Note that the two groups execute in parallel which means there is no guarantee
to their order of execution.
However, the synchronization ensures that the reads see a consistent set of
writes in the order we expect.

## Limitations

The example above implements a standard producer-consumer.
However, as implemented, the `std_sync_reg` primitive does not support multiple
producers or consumers.
To do so, it would need to provide an interface that allows several read and
write ports and ensure that only one read or write operation succeeds.
This capability would be useful in implementing synchronizing barriers in Calyx.


[par-undef]: ./undefined.md#semantics-of-par
[m-struct]: http://composition.al/blog/2013/09/22/some-example-mvar-ivar-and-lvar-programs-in-haskell/
[ex]: https://github.com/cucapra/calyx/blob/master/tests/correctness/sync.futil
