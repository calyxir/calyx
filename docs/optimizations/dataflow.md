# Dataflow Optimizations
In general, dataflow analysis uses the control and data flow of a program to compute
various properties (liveness, reaching definitions, ...) at each point in a program.

For FuTIL, dataflow analyses use the explicit control program and knowledge about
the dataflow of each group to compute properties about each group.

## Basic blocks vs. Groups
Normally, dataflow analyses compute a property at each basic block of a control
flow graph (CFG). FuTIL doesn't have a notion of basic blocks, and so FuTIL computes
a property at each group in a program.

Because FuTIL separates the control flow of a program from the specification of
groups, it's possible for a group to appear multiple times in the control program.
For this reason we compute a property at each group *enable* rather than each group
*definition*. The property at each group *definition* can easily be computed
as the meet over all group enables.

## Dataflow on an AST
Dataflow analyses are typically performed by finding the fixed point
of a set of equations defined at each node of a control flow graph (CFG)
using the [worklist algorithm][].

Because our control AST is little more than just the edges of a [reducible cfg][],
we don't bother to build an explicit CFG and instead perform the
dataflow analysis directly on the AST using FuTIL's visitor infrastructure.

### Abstract Algorithm
For some property `p`, for each control statement, `s`, we define `dfa(s, p ins)`
 - when `s = enable A`: `dfa(enable A, p) = transfer(A, p)`
 - when `s = seq { A; B; ...; Z; }`:\
 `dfa(seq { A; B; ...; Z; }, p) = dfa(A, p) |> dfa(B, _) |> ... |> dfa(Z; _)`

### Equivalence to worklist algorithm
In the normal worklist algorithm, we add a statement back to the worklist when
it's predecessor has changed.

## Parallel Dataflow


[worklist algorithm]: https://en.wikipedia.org/wiki/Data-flow_analysis#An_iterative_algorithm
[reducible cfg]: https://en.wikipedia.org/wiki/Control-flow_graph#Reducibility
