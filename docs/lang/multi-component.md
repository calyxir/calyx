# Multi-Component Designs

Calyx designs can define and instantiate other Calyx components that themselves
encode complex `control` programs.

As an example, we'll build a Calyx design that uses a simple Calyx component
to save a value in a register and use it in a different component.

We define a new component called `identity` that has an input port `in`
and an output port `out`.

```
{{#include ../../examples/futil/multi-component.futil:component}}
```

The following line defines a *continuous assignment*, i.e., an assignment
that is always kept active, regardless of the component's `control` program
being active.

```
{{#include ../../examples/futil/multi-component.futil:wires}}
```

By defining this continuous assignment, we can *execute* our component and
later observe any relevant values.

Next, we can instantiate this component in any other Calyx component.
The following Calyx program instantiates the `id` component and uses it to
save a value and observe it.

```
{{#include ../../examples/futil/multi-component.futil:main}}
```

Our first group executes the component by setting the `go` signal for the
component to high and placing the value `10` on the input port.
The second group simply saves the value on the output port. Importantly,
we don't have to set the `go` signal of the component to high because we
don't need to save a new value into it.
The component executes the two groups in-order.

To see the output from running this component, run the command:
```
fud e examples/futil/multi-component.futil --to vcd
```
