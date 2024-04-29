# Static Timing

> The features discussed below have been available since Calyx version 0.2.1.

By default, Calyx programs use a *latency-insensitive*, or *dynamic*, model of computation.
This means that the compiler does not know, track, or guarantee the number of cycles it takes to perform a computation or run a control operator.
This is in contrast to a *latency-sensitive*, or *static*, model of computation, where the number of cycles a component needs is known to, and honored by, the compiler.

In general, latency-insensitivity makes it easier to compose programs.
It grants the compiler freedom to schedule operators however it wants, as long as the schedule meets the program's dataflow constraints.
It also prevents code from implicitly depending on the state of other code running in parallel.

However, there are two drawbacks to this approach.
First, the generated hardware may not be efficient: if the compiler does not know how long computations take, it must schedule them conservatively.
Second, it is impossible for *latency-insensitive* programs to interact with *latency-sensitive* hardware implemented in RTL;
this means that the use of black-box hardware designs requires costly handshaking logic at the interface.

To address these issues, Calyx provides a `static` qualifier that modifies components and groups, along with static variants of other control operators.

Broadly, the `static` qualifier is a promise to the compiler that the specifed component or group will take exactly the specified number of cycles to execute.
The compiler is free to take advantage of this promise to generate more efficient hardware.
In return, the compiler must access out-ports of static components only after the specified number of cycles have passed, or risk receiving incorrect results.

## Static Constructs in the Calyx IL

We will now discuss the static constructs available in the Calyx IL, along with the guarantees they come with.

### Static Components

Briefly consider a divider component, `std_div`, which divides the value `left` by the value `right` and puts the result in `out`.
This component is dynamic; its latency is unknown.
```
primitive std_div[W](go: 1, left: W, right: W) -> (out: W, done: 1);
```
A client of the divider must pass two inputs `left` and `right`, raise the `go` signal, and wait for the component itself to raise its `done` signal.
The client can then read the result from the `out` port.
That is, it obeys the [go-done interface][go-done-interface].

Compare this to a multiplier component, `std_mult`, which has a similar signature but whose latency is known to be three cycles.
We declare it as follows:
```
static<3> primitive std_mult[W](go: 1, left: W, right: W) -> (out: W);
```

The key differences are:
- The `static` qualifier is used to declare the component as static and to specify its latency (3 cycles).
- The `done` port is absent.

A client of the multiplier must pass two inputs and raise the `go` signal as before.
However, the client need not then wait for the component to indicate completion.
It can simply and safely assume that the result will be available after 3 cycles.
This is a guarantee that the author of the component has made to the client, and the compiler is free to take advantage of it.


### Static Groups and Relative Timing Guards

Much like components, groups can be declared as static.
Since groups are just unordered sets of assignments, it pays to have a little more control over the scheduling of the assignments within a group.
To this end, static groups have a unique feature that ordinary dynamic groups do not: *relative timing guards*.

Consider this group, which multiplies `6` and `7` and stores the result in `ans`.

```
static<4> group mult_and_store {
  mult.left = %[0:3] ? 6;
  mult.right = %[0:3] ? 7;
  mult.go = %[0:3] ? 1;
  ans.in = %3 ? mult.out;
  ans.write_en = %3 ? 1;
}
```
The `static<4>` qualifier specifies that the group should take 4 cycles to execute.

The first three assignments are guarded (using the [standard `?` separator][guard-sep]) by the relative timing guard `%[0:3]`.
In general, a relative timing guard `%[i:j]` is *true* in the half-open interval from cycle `i` to
cycle `j` of the group’s execution and *false* otherwise.

In our case, the first three assignments execute only in the first three cycles of the group's execution.
The guard `%3`, which we see immediately afterwards, is syntactic sugar for `%[3:4]`.
We have used it in this case to ensure that the last two assignments execute only in the last cycle of the group's execution.


### Static Control Operators

Calyx provides static variants of each of its [control operators][].
While dynamic commands may contain both static and dynamic children, static commands must only have static children.
In the examples below, assume that `A5`, `B6`, `C7`, and `D8` are static groups with latencies 5, 6, 7, and 8, respectively.

#### `static seq`, a static version of [`seq`][seq]
If we have `static seq { A5; B6; C7; D8; }`, we can guarantee that the latency of the entire operation is the sum of the latencies of its children: 5 + 6 + 7 + 8 = 26 cycles in this case.
We can also guarantee that each child will begin executing exactly one cycle after the previous child has finished.
In our case, for example, `B6` will begin executing exactly one cycle after `A5` has finished.

#### `static par`, a static version of [`par`][par]
If we have `static par { A5; B6; C7; D8; }`, we can guarantee that the latency of the entire operation is the maximum of the latencies of its children: 8 cycles in this case.
Further, all the children of a `static par` are guaranteed to begin executing at the same time.
The children can rely on this "lockstep" behavior and can communicate with each other.
Inter-thread communication of this sort is undefined behavior in a standard, dynamic, `par`.

As a corollary, consider this useful trick in the case when we need `A5` and `D8` to run in parallel, but we do not want them to start at the same time.
Instead, in order to support some inter-thread communication, we want `A5` to start three cycles after `D8`.

```
static<3> group dummy_group { }

static par {
  static seq { dummy_group; A5; }
  D8;
}
```
We have not elided the body of `dummy_group`; it can literally be left blank.
During compilation, no hardware will be generated for `dummy_group`. It is simply a placeholder to delay the start of `A5` by three cycles.

#### `static if`, a static version of [`if`][if]
If we have `static if { A5; B6; }`, we can guarantee that the latency of the entire operation is the maximum of the latencies of its children: 6 cycles in this case.


#### `static repeat`, a static version of [`repeat`][repeat]

If we have `static repeat 7 { B6; }`, we can guarantee that the latency of the entire operation is the product of the number of iterations and the latency of its child: 7 × 6 = 42 cycles in this case.
The body of a `static repeat` is guaranteed to begin executing exactly one cycle after the previous iteration has finished.

Calyx's [`while`][while] loop is unbouded and so it does not have a static variant.

#### `static invoke`, a static version of [`invoke`][invoke]

Its latency is the latency of the invoked cell.

[guard-sep]: ./ref.md#guarded-assignments
[go-done-interface]: ./ref.md#the-go-done-interface
[control operators]: ./ref.md#the-control-operators
[seq]: ./ref.md#seq
[par]: ./ref.md#par
[if]: ./ref.md#if
[repeat]: ./ref.md#repeat
[invoke]: ./ref.md#invoke
[while]: ./ref.md#while
