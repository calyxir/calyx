# Calyx Language Reference

## Top-Level Constructs

Calyx programs are a sequence of `import` statements followed by a sequence of
`extern` statements or `component` definitions.

### `import` statements

`import "<path>"` has almost exactly the same semantics to that of `include` in
C-style programming languages; it copies the code `path` into the current file.

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
When run with the `-b verilog` flag, the calyx compiler will copy the text contained
in every such verilog file into the generated output.

### `primitive` definitions

The `primitive` construct allows specification of the signature of an external
Verilog module that the Calyx program uses.
It has the following syntax:
```
[comb] primitive name<attributes>[PARAMETERS](ports) -> (ports);
```

- The `comb` primitive is used to signal that the primitive definition wraps purely
  combinational RTL code. This is useful for certain optimizations.
- The *attributes* syntax allows specification of useful optimization metadata in
  form of [attributes][].
- *PARAMETERS* defines the parameters passed to the RTL code.
- The *ports* section contain sized port definitions that can either be positive number
  or one of the parameters.

For example, the following is the signature of the `std_reg` primitive from the
Calyx standard library:
```
{{#include ../../primitives/core.futil:std_reg_def}}
```

The primitive defines one parameter called `WIDTH` which describes the sizes for
the `in` and the `out` port.

## Calyx Components

`components` are the primary encapsulation unit of a Calyx program.
They look like this:
```
component name<attributes>(ports) -> (ports) {
  cells { .. }
  wires { .. }
  control { .. }
}
```

Unlike `primitive` definitions, `component` definitions cannot be parameterized and
must provide exact port width for all ports.
A component encapsulates the control and the hardware structure needed to implement
a hardware module.

### `ports`

Ports defined in Calyx only specify their bitwidth and are otherwise untyped:
```
component counter(left: 32, right: 32) -> (@stable out0: 32, out1: 32) { .. }
```
The component defines two input ports `left` and `right` and two output ports
`out0` and `out1`.
Additionally, the `out0` port has the [attribute][attributes] `@stable`.

### `cells`

The `cells` section of the Calyx program instantiates all the sub-component used
by this component.
For example, the following definition of the `counter` component instantiates a
`std_add` and `std_reg` primitive as well as a `foo` calyx component
```
component foo() -> () { .. }
component counter() -> () {
  cells {
    r = std_reg(32);
    a = std_add(32);
    f = foo();
  }
  wires { .. }
  control { .. }
}
```

When instantiating a `primitive` definition, the parameter are passed within the
parenthesis.
For example, we pass `32` for the `WIDTH` parameter of the `std_reg` in the above
instantiation.
Since an instantiation of a calyx component does not take any parameters, the parameters
are always empty.

## The `wires` Section

A component's `wires` section is a sequence of guarded assignments, `group` definitions, or `comb group` definitions.

### Continuous assignments

Assignments connect ports between two cells together:
```
r.in = add.out;
```
The above assignment *continuously* transfer the value in `add.out` to `r.in`.

Because assignments are *continuous*, their order does not matter:
```
r.in = add.out;
add.left = y.out;
```

Assignments can additionally be guarded using a 1-bit value:
```
r.in = cond.out ? add.out;
r.in = !cond.out ? 32'd0;
```

Guards allow specification of multiple different values to a port.

> **Well-formedness**: For each input port on the LHS, only one guard should be active in any given cycle during the execution of a Calyx program.

When an assignment is directly placed into a component's `wires` section, it
is called a "continuous assignment" and is permanently active, even when the
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
These means that seemingly conflicting writes to the same ports are allowed:
```
group foo {
  r.in = 32'd10; ..
  foo[done] = ..
}
group bar {
  r.in = 32'd22; ..
  bar[done] = ..
}
```
This is because group only execute when they are mentioned in the [control program](#the-control-operators).

However, group assignments are not allowed to conflict with [continuous assignments](#continuous-assignments) defined in the component:
```
group foo {
  r.in = 32'd10; .. // This is malformed because it conflicts with the write below
  foo[done] = ..
}
r.in = 32'd50;
```

The `done` condition of a group is any 1-bit port that, when set to 1, represents the completion of a group's execution.
This is because groups can take any number of cycles and therefore need a way
to specify to the external world when their execution has completed.

The attributes syntax specifies the attributes for the group.

> **Well-formedness**: All groups are required to run for at least one cycle.

### `comb group` definitions

Combinational groups are a restricted version of groups which perform their
computation purely combinationally and therefore run for "less than one cycle":
```
comb group name<attributes> {
  assignments..
}
```

Because their computation is required to run for less than a cycle, `comb group`
definitions do not specify a `done` condition.

Combinational groups cannot be used within normal [control
operators](#the-control-operators).
Instead, they only occur after the `with` keyword in a control program.


## The Control Operators

The `control` section of a component contains a control program built-up using
the following operators:

### Group enable

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
  group](#comb-group) that is active during the execution of the `invoke`
  statement.
- The [`ref cells`](#ref-cells) syntax is described in detail

Invoking a instance runs its control program to completion before returning.
Any Calyx component or primitive that implements the [go-done interface](#the-go-done-interface) can be invoked.

### `seq`

The syntax is:
```
seq { c1; .. cn; }
```

Sequences run the control programs, `c1`..`cn` in sequence, guaranteeing that
each program runs fully before the next one starts executing.
`seq` **does not** provide any cycle-level guarantees on when a succeeding
group starts executing.

### `par`

The syntax is:
```
par { c1; .. cn; }
```

Parallel runs the control programs, `c1`..`cn` in parallel, guaranteeing that
each program only runs once.
`par` **does not** provide any guarantees on how the execution of child programs
is scheduled.
It is therefore not safe to assume that all children begin execution at the
same time.

> **Well-formedness**: The assignments in the children `c1`..`cn` should never conflict with each other.

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
The optional `with comb_group` syntax allows running a combinational group
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
The optional `with comb_group` enables a combinational group that computes the
value of `port`.

> **Well-formedness**: The combinational group is considered active during the execution of the while
> loop and therefore should not have conflicting assignments with `body_c`.

## The `go`-`done` Interface

By default, calyx components implement a one-sided ready-valid interface called
the `go`-`done` interface.
During compilation, the Calyx compiler will add an input port marked with the attribute `@go` and an output port marked with the attribute `@done` to the interface of the component:
```
component counter(left: 32, right: 32, @go go: 1) -> (out: 32, @done done: 1) ..
```

The interface provides a way to trigger the control program of the counter using
assignments.
When the `go` signal of the component is set to 1, the control program starts
executing.
When the component sets the `done` signal to 1, its control program has finished
executing.

> **Well-formedness**: The `go` signal to the component must remain set to 1 while the done signal is not 1. Lowering the `go` signal before the `done` signal is set to 1 will lead to undefined behavior.

## The `clk` and `reset` Ports

The compiler also adds special input ports marked with `@clk` and `@reset` to the
interface.
By default, the Calyx components are not allowed to look at, or use these signal.
They are automatically threaded to any primitive that defines `@clk` or
`@reset` ports.


## Advanced Concepts

### `ref` cells

**TK**

[attributes]: ./attributes.md
