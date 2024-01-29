# Static Timing

By default, Calyx programs use a *latency-insensitive*, or *dynamic*, model of computation.
This means that the compiler does not know, track, or guarantee the number of cycles it takes to perform a computation or run a control operator.
This is in contrast to a *latency-sensitive*, or *static*, model of computation, where the number of cycles a component needs is known to, and honored by, the compiler.

In general, latency-insensitivity makes it easier to compose programs.
It grants the compiler freedom to schedule operators however it wants, as long as it meets the program's dataflow constraints.

However, there are two drawbacks to this approach.
First, the generated hardware may not be efficient: if the compiler does not know how long computations take, it must schedule them conservatively.
Second, it is impossible for *latency-insensitive* programs to interact with *latency-sensitive* hardware implemented in RTL;
this prevents the use of black-box hardware designs.

To address these issues, Calyx provides a `static` keyword that modifies components and groups, along with static variants of other control operators.

## Static support in the Calyx IL

### Static components

Say we have a multiplier component, `std_mult`, which multiplies the values `left` and `right` and puts the result in `out`.
Its latency is 3 cycles.
We can declare it as follows:
```
static<3> primitive std_mult[W](go: 1, left: W, right: W) -> (out: W);
```
Compare this to the divider component `std_div`, whose latency is unknown:
```
primitive std_div[W](go: 1, left: W, right: W) -> (out: W, done: 1)
```
The key differences are:
- The `static` keyword is used to declare the component as static and to specify its latency.
- The `done` port is not present in the static component.

A client of the divider must pass two inputs, raise the `go` signal, and wait for the component itself to raise its `done` signal.
In contrast, a client of the multiplier must pass two inputs and raise the `go` signal, but it does not need to wait for the component to raise a `done` signal.
It can simply and safely assume that the result will be available after 3 cycles.


### Static groups and relative timing guards

Much like components, groups can be declared as static.
Since groups are just unordered sets of assignments, it pays to have a little more control over the scheduling of the assignments within a group.
Consider this group, which performs `ans := 6 * 7`:
```
static<4> group mult_and_store {
  mult.left = %[0:3] ? 6;
  mult.right = %[0:3] ? 7;
  mult.go = %[0:3] ? 1;
  ans.in = %3 ? mult.out;
  ans.write_en = %3 ? 1;
}
```
The `static<4>` keyword specifies that the group should take 4 cycles to execute.
However, we cannot just let all the assignments execute in any order, as we could with a dynamic group.

The first three assignments are guarded (using the standard `?` operator) by the *relative timing guard* `%[0:3]`.
That is, they execute only in the first three cycles of the group's execution.
The guard `%3`, which we see thereafter, is syntactic sugar for `%[3:4]`.
In general, a guard `%[i:j]` is true in the half-open interval from cycle `i` to
cycle `j` of the group’s execution.

### Static control operators

Calyx provides static variants of each of its control operators.
While dynamic commands may contain both static and dynamic children, static commands must only have static children.

- `static_seq` is a static version of `seq`; its latency is the sum of the latencies of its children.
- `static_par` is a static version of `par`; its latency is the maximum of the latencies of its children.
- `static_if` is a static version of `if`; its latency is the maximum of the latencies of its children.
- Calyx's `while` loop is unbouded, so it does not have a static variant.
- `static_repeat` is a static version of `repeat`; its latency is the product of the number of iterations and the latency of its child.
- `static_invoke` is a static version of `invoke`; its latency is the latency of the invoked cell.

## Guarantees

The `static` keyword is a promise to the compiler that the component or group will take exactly the specified number of cycles to execute.
The compiler is free to take advantage of this promise to generate more efficient hardware.
In return, the compiler must access out-ports of static components only after the specified number of cycles have passed, or risk receiving incorrect results.
