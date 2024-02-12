# Calyx Language Reference

## Top-Level Constructs

Calyx programs are a sequence of `import` statements followed by a sequence of
`extern` statements or `component` definitions.

### `import` statements

`import "<path>"` has almost exactly the same semantics to that of `#include` in
the C preprocessor: it copies the code from the file at `path` into
the current file.

### `extern` definitions

`extern` definitions allow Calyx programs to link against arbitrary RTL code.
An `extern` definition looks like this:
```
extern "<path>" {
  <primitives>...
}
```

`<path>` should be a valid file system path that points to a Verilog module that
defines the same names as the *primitives* defined in the `extern` block.
When run with the `-b verilog` flag, the Calyx compiler will copy the contents
of every such Verilog file into the generated output.

### `primitive` definitions

The `primitive` construct allows specification of the signature of an external
Verilog module that the Calyx program uses.
It has the following syntax:

```
[comb] primitive name<attributes>[PARAMETERS](ports) -> (ports);
```

The syntax for primitives resembles that for [components][],
with some additional pieces:

- The `comb` keyword signals that the primitive definition wraps purely
  combinational RTL code. This is useful for certain optimizations.
- *[Attributes][]* specify useful metadata for optimization.
- *PARAMETERS* are named compile-time (metaprogramming) parameters to pass to
  the Verilog module definition. The primitive definition lists the names of
  integer-valued parameters; the corresponding Verilog module definition should
  have identical `parameter` declarations. Calyx code provides values for these
  parameters when instantiating a primitive as a [cell][cells].
- The *ports* section contain sized port definitions that can either be positive number
  or one of the parameter names.

For example, the following is the signature of the `std_reg` primitive from the
Calyx standard library:
```
{{#include ../../primitives/compile.futil:std_reg_def}}
```

The primitive defines one parameter called `WIDTH`, which describes the sizes for
the `in` and the `out` ports.

### Inlined Primitives

*Inlined primitives* do not have a corresponding Verilog file, and are defined within Calyx. The Calyx backend then converts these definitions into Verilog.

For example, the `std_unsyn_mult` primitive is inlined:
```
{{#include ../../primitives/unsynthesizable.futil:std_unsyn_mult_def}}
```

This can be useful when a frontend needs to generate both Calyx and Verilog code at the same time. The backend ensures that the generated Verilog module has the correct signature.

## Calyx Components

Components are the primary encapsulation unit of a Calyx program.
They look like this:

```
component name<attributes>(ports) -> (ports) {
  cells { ... }
  wires { ... }
  control { ... }
}
```

Like [`primitive` definitions][prim], `component` signatures consist of a name, an optional list of attributes, and input/output ports.
Unlike `primitive`s, `component` definitions do not have parameters; ports must have a concrete (integer) width.
A component encapsulates the control and the hardware structure that implements
a hardware module.

> **Well-formedness**: The `control` program of a component must take at least one cycle to finish executing.

### Combinational Components

Using the `comb` keyword before a component definition marks it as a purely combinational component:
```
comb component add(left: 32, right: 32) -> (out: 32) {
  cells {
    a = std_add(32);
  }
  wires {
    a.left = left;
    a.right = right;
    out = a.out;
  }
}
```

A combinational component does not have a `control` section, can only use other `comb` components or primitives, and performs its computation combinationally.

### Ports

A port definition looks like this:

```
[@<attr>...] <name>: <width>
```

Ports have a bit width but are otherwise untyped.
They can also include optional [attributes][].
For example, this component definition:

```
component counter(left: 32, right: 32) -> (@stable out0: 32, out1: 32) { .. }
```

defines two input ports, `left` and `right`, and two output ports,
`out0` and `out1`.
All four ports are 32-bit signals.
Additionally, the `out0` port has the [attribute][attributes] `@stable`.

### `cells`

A component's `cells` section instantiates a set of sub-components.
It contains a list of declarations with this syntax:

```
[ref]? <name> = <comp>(<param...>);
```

Here, `<comp>` is the name of an existing [primitive][prim] or [component definition][components], and
`<name>` is the fresh, local name of the instance.
The optional `ref` parameter turns the cell into a [by-reference cell](#ref-cells).
Parameters are only allowed when instantiating primitives, not Calyx-defined components.

For example, the following definition of the `counter` component instantiates a
`std_add` and `std_reg` primitive as well as a `foo` Calyx component

```
component foo() -> () { ... }
component counter() -> () {
  cells {
    r = std_reg(32);
    a = std_add(32);
    f = foo();
  }
  wires { ... }
  control { ... }
}
```

When instantiating a [`primitive` definition][prim], the parameters are passed within the
parenthesis.
For example, we pass `32` for the `WIDTH` parameter of the `std_reg` in the above
instantiation.
Since an instantiation of a Calyx component does not take any parameters, the parameters
are always empty.

## The `wires` Section

A component's `wires` section contains *guarded assignments* that connect ports
together. The assignments can either appear at the top level, making them
*[continuous assignments][continuous]*, or be organized into named
[`group`][group] and [`comb group` definitions][comb].

### Guarded Assignments

Assignments connect ports between two cells together, with this syntax:

```
<cell>.<port> = [<guard> ?] <cell>.<port>;
```

The left-hand and right-hand side are both *port references*, which name a
specific input or output port within a [cell][cells] declared within the same
component. The optional *guard condition* is a logical expression that
determines whether the connection is active.

For example, this assignment:

```
r.in = add.out;
```

unconditionally transfers the value from a port named `out` in the `add` cell to `r`'s `in` port.

Assignments are *simultaneous* and *non-blocking*. When a block of assignments
runs, they all first read their right-hand sides and then write into their
left-hand sides; they are not processed in order. The result is that the order
of assignments does not matter. For example, this block of assignments:

```
r.in = add.out;
add.left = y.out;
add.right = z.out;
```

is a valid way to take the values from registers `y` and `z` and put the sum into `r`. Any permutation of these assignments is equivalent.

### Guards

An assignment's optional *guard* expression is a logical expression that produces a 1-bit value, as in these examples:

```
r.in = cond.out ? add.out;
r.in = !cond.out ? 32'd0;
```

Using guards, Calyx programs can assign multiple different values to the same
port. Omitting a guard expression is equivalent to using `1'd1` (a constant
"true") as the guard.

Guards can use the following constructs:
- `port`: A port access on a defined cell
- `port op port`: A comparison between values on two ports. Valid `op` are: `>`, `<`, `>=`, `<=`, `==`
- `!guard`: Logical negation of a guard value
- `guard || guard`: Disjunction between two guards
- `guard && guard`: Conjunction of two guards

In the context of guards, a port can also be a literal (i.e., `counter.out == 3'd2` is a valid guard).

> **Well-formedness**: For each input port on the LHS, only one guard should be active in any given cycle during the execution of a Calyx program.

### Continuous Assignments

When an assignment appears directly inside a component's `wires` section, it
is called a *continuous assignment* and is permanently active, even when the
[control program](#the-control-operators) of the component is inactive.

### `group` definitions

A `group` is a way to name a set of assignments that together represent some
meaningful action:

```
group name<attributes> {
  assignments...
  name[done] = done_cond;
}
```

Assignments within a group can be reasoned about in isolation from assignments
in other groups.
Unlike [continuous assignments][continuous], a group's encapsulated assignments
only execute as dictated by the [control program][control].
This means that seemingly conflicting writes to the same ports are allowed:

```
group foo {
  r.in = 32'd10;
  foo[done] = ...;
}
group bar {
  r.in = 32'd22;
  bar[done] = ...;
}
```

However, group assignments must not conflict with [continuous assignments][continuous] defined in the component:

```
group foo {
  r.in = 32'd10; ... // Malformed because it conflicts with the write below.
  foo[done] = ...
}
r.in = 32'd50;
```

Groups can take any (nonzero) number of cycles to complete. To indicate to the
outside world when their execution has completed, every group has a special
*done signal*, which is a special port written as `<group>[done]`. The group
should assign 1 to this port to indicate that its execution is complete.

Groups can have an optional list of [attributes][].

> **Well-formedness**: All groups are required to run for at least one cycle. (Sub-cycle logic should use `comb group` instead.)

### `comb group` definitions

Combinational groups are a restricted version of groups which perform their
computation purely combinationally and therefore run for "less than one cycle":

```
comb group name<attributes> {
  assignments...
}
```

Because their computation is required to run for less than a cycle, `comb group`
definitions do not specify a `done` condition.

Combinational groups cannot be used within normal [control
operators][control].
Instead, they only occur after the `with` keyword in a control program.


## The Control Operators

The `control` section of a component contains an imperative program that
describes the component's behavior. The statements in the control program
consist of the following operators:

### Group Enable

Simply naming a group in a control statement, called a group enable, executes
the group to completion.
This is a leaf node in the control program.

### `invoke`

`invoke` acts like the function call operator for Calyx and has the following
syntax:

```
invoke instance[ref cells](inputs)(outputs) [with comb_group];
```

- `instance` is the name of the cell instance that needs to be invoked.
- `inputs` and `outputs` define connections for a subset of input and output
  ports of the instance.
- The `with comb_group` section is optional and names a [combinational
  group][comb] that is active during the execution of the `invoke`
  statement.
- `ref cells` is a list of cell names to pass to the invoked component's
  required [*cell reference*][ref]. (It can be omitted if the invoked component
  contains no cell references.)

Invoking an instance runs its control program to completion before returning.
Any Calyx component or primitive that implements the [go-done interface](#the-go-done-interface) can be invoked.
Like the [group enable](#group-enable) statement, `invoke` is a leaf node in the control program.

### `seq`

The syntax for sequential composition is:

```
seq { c1; ...; cn; }
```

where each `ci` is a nested control statement.
Sequences run the control programs `c1`...`cn` in sequence, guaranteeing that
each program runs fully before the next one starts executing.
`seq` **does not** provide any cycle-level guarantees on when a succeeding
group starts executing after the previous one finishes.

### `par`

The syntax for parallel composition is:

```
par { c1; ...; cn; }
```

The statement runs the nested control programs `c1`...`cn` in parallel, guaranteeing that
each program only runs once.
`par` **does not** provide any guarantees on how the execution of child programs
is scheduled.
It is therefore not safe to assume that all children begin execution at the
same time.

> **Well-formedness**: The assignments in the children `c1`...`cn` should never conflict with each other.

### `if`

The syntax is:
```
if <port> [with comb_group] {
  true_c
} else {
  false_c
}
```

The conditional execution runs either `true_c` or `false_c` using the value of
`<port>`.
The optional `with comb_group` syntax allows running a [combinational group][comb]
that computes the value of the port.

> **Well-formedness**: The combinational group is considered to be running during the entire execution
> of the control program and therefore should not have conflicting assignments
> with either `true_c` or `false_c`.

### `while`

The syntax is:
```
while <port> [with comb_group] {
  body_c
}
```

Repeatedly executes `body_c` while the value on `port` is non-zero.
The optional `with comb_group` enables a [combinational group][comb] that computes the
value of `port`.

> **Well-formedness**: The combinational group is considered active during the execution of the while
> loop and therefore should not have conflicting assignments with `body_c`.

### `repeat`

The syntax is:
```
repeat <num_repeats> {
  body_c
}
```

Repeatedly executes the control program `body_c` `num_repeat` times in a row.


## The `go`-`done` Interface

By default, Calyx components implement a one-sided ready-valid interface called
*the `go`-`done` interface*.
During compilation, the Calyx compiler will add an input port marked with the attribute [`@go`][godoneattr] and an output port marked with the attribute [`@done`][godoneattr] to the interface of the component:

```
component counter(left: 32, right: 32, @go go: 1) -> (out: 32, @done done: 1)
```

The interface provides a way to trigger the control program of the counter using
assignments.
When the `go` signal of the component is set to 1, the control program starts
executing.
When the component sets the `done` signal to 1, its control program has finished
executing.

> **Well-formedness**: The `go` signal to the component must remain set to 1 while the done signal is not 1. Lowering the `go` signal before the `done` signal is set to 1 will lead to undefined behavior.

## The `clk` and `reset` Ports

The compiler also adds special input ports marked with [`@clk` and `@reset`][clkreset] to the
interface.
By default, Calyx components are not allowed to look at or use these signals.
They are automatically threaded to any primitive that defines `@clk` or
`@reset` ports.


## Advanced Concepts

### `ref` cells

Calyx components can specify that a cell needs to be passed "by reference":

```
// Component that performs mem[0] += 1;
component update_memory() -> () {
  cells {
    ref mem = comb_mem_d1(...)
  }
  wires { ... }
  control { ... }
}
```

When invoking such a component, the calling component must provide a binding for each defined cell:
```
component main() -> () {
  cells {
    upd = update_memory();
    m1 = comb_mem_d1(...);
    m2 = comb_mem_d2(...);
  }
  wires { ... }
  control {
    seq {
      invoke upd[mem=m1]()(); // Pass `m1` by reference
      invoke upd[mem=m2]()(); // Pass `m2` by reference
    }
  }
}
```
As the example shows, each invocation can take different bindings for each `ref` cell.
See [the tutorial][ref-tut] for longer example on how to use this feature.

[attributes]: ./attributes.md
[components]: #calyx-components
[cells]: #cells
[prim]: #primitive-definitions
[group]: #group-definitions
[comb]: #comb-group-definitions
[continuous]: #continuous-assignments
[control]: #the-control-operators
[ref]: #ref-cells
[godoneattr]: ./attributes.md#go-done-clk-and-reset
[clkreset]: ./attributes.md#go-done-clk-and-reset
[ref-tut]: ./memories-by-reference.md
