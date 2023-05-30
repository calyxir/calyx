# Builder Library Reference

## Top-Level Program Structure

Here's the general structure of a program that uses the builder to generate Calyx code.

```python
# import the builder library
import calyx.builder as cb


# define `second_comp`
def add_second_comp(prog):
    # `second_comp` definition here


# method for defining `my_component` and adding it to a program
def add_my_component(prog, second_comp): 
    # add the component to the program
    my_component = prog.component("my_component")

    # Adding an instance of `second_comp` as a cell of `my_component`
    my_second_comp = my_component.cell("my_second_comp", second_comp)

    # adding a register cell (or other cells) to the component 
    my_reg = my_component.reg("my_reg", 32)

    # define a `my_component` group
    with my_component.group("my_group") as my_group:
      # assignments here 
      my_reg.write_en = 1

    # add the group to `my_component`'s control program
    my_component.control += my_group


# assemble the program
def build():
    prog = cb.Builder()
    my_second_comp = add_second_comp(prog)
    add_my_component(prog, my_second_comp)

    # return the generated program
    return prog.program


# emit the program
if __name__ == "__main__":
    build().emit()
```

## Components

### Defining Components

To define a component, call the `Builder().component()` method.

```python
prog = cb.Builder()
prog.component("my_component")
```

### Retrieving Components

To reference a component without an existing [handle][hndl] to it, use the `Builder().get_component()` method.

```python
prog = cb.Builder()
prog.component("my_component")
# a few lines later 
my_component = prog.get_component("my_component")
```

### Defining Component Inputs and Outputs

Components can be given input and output ports. Just specify the name of the port and its size.

```python
my_component.input("my_input", 32)
my_component.output("my_output", 32)
```

To access the input and output ports of a component within the definition of a component, use the syntax `my_component.this().port`.

```python
def add_my_component(prog):
    my_component = prog.component("my_component")
    my_component.output("my_output", 32)

    with my_component.group("my_group"):
        my_component.this().my_output = const(32, 1)
```

Note that it's possible to [create a handle][hndl] to input and output ports.

### Multi-Component Designs

Calyx supports [multi-component designs][multi]. The [top-level example][top] demonstrates how to construct multi-component designs using the library.

#### Defining Common Calyx Cells

Here's a snippet of code that adds a few common kinds of cells to a component:

```python
my_component = prog.component("my_component")

# Registers: reg(name, bitwidth)
my_component.reg("my_reg", 32)

# Constants: const(name, bitwidth, value)
my_component.const("my_reg", 32, 42)

# Adders/Subtractors: [add|sub](name, size, signed=False)
# a signed adder 
my_component.add("my_add", 32, signed=True)
# a subtractor
my_component.sub("my_sub", 32)


# Comparators: [gt|lt|eq|neq|ge|le](name, size, signed=False)
my_component.gt("my_gt", 32)
# a signed le comparison
my_component.lt("my_lt", 32, signed=True)
my_component.eq("my_eq", 32)
my_component.neq("my_neq", 32)
my_component.ge("my_ge", 32)
my_component.le("my_le", 32)

# 1-D memory: 
# mem_d1(name, bitwidth, len, idx_size, is_external=False, is_ref=False)
my_component.mem_d1("my_mem", 32, 4, 32)
# An external memory
my_component.mem_d1("my_mem", 32, 4, 32, is_external=True)
# A memory by reference
my_component.mem_d1("my_mem", 32, 4, 32, is_ref=True)
```

If you're curious, you can read more about [external memories][ext] or [memories by reference][ref].

#### Retrieving Cells

In order to reference a cell without a [handle][hndl], use the `Builder().get_cell()` method.

```python
# defining a register cell
my_component.reg("my_reg", 32)

# a few lines later 
my_reg = prog.get_cell("my_reg")
```

## Wires

### Guarded Assignments

Guarded assignments in the builder are syntactically similar to those in Calyx.

```python
my_component = prog.component("my_component")

my_add = comp.add("my_add", 32)
my_reg = comp.reg("my_reg", 32)

with my_component.group("my_group"):
    # unconditional assignments
    add.left = const(32, 1)
    add.right = const(32, 41)
    my_reg.write_en = 1

    # a guarded assignment using @
    # in Calyx, this line would be:
    # my_reg.in = (add.left < add.right) ? add.out;
    my_reg.in = (add.left < add.right) @ add.out
```

### Groups

#### Defining Groups

[Groups][grp] are defined using the `group()` method for a component. To make a [handle][hndl] for a group, use the `as` syntax. Group handles are necessary in order to set done ports for groups.

It's possible to define a [static delay][static] for a group using the optional `static_delay` argument.

```python
my_component = prog.component("my_component")

# a group definition + handle for the group
with my_component.group("my_group") as my_group:
    # assignments here

# a group with a static delay
with my_component.group("my_static_group", static_delay=1): 

```

#### Defining Combinational Groups

[Combinational groups][comb] are defined similarly to groups, except with the `comb_group` method.

```python
my_component = prog.component("my_component")

with my_component.comb_group("my_comb_group"):
    # assignments here
```

#### Retrieving Groups

If a group doesn't have a [handle][hndl], it can be retrieved later with the `Builder().get_group()` method. It's possible to retrieve combinational groups as well as regular groups with this method.

```python
prog = cb.Builder()
my_component = prog.component("my_component")

with my_component.group("my_group"):
    # group definition here

# a few lines later
my_group = prog.get_group("my_group")

with my_component.comb_group("my_comb_group"):
    # comb group definition here

my_comb_group = prog.get_group("my_comb_group")

```

### Continuous Assignments

[Continuous assignments][cont] are generated by using the syntax `with comp.continuous`.

```python
my_component = prog.component("my_component")

my_output = my_component.output("my_output", 32)
my_reg = comp.reg("my_reg", 32)

with my_component.continuous:
    my_component.this().my_output = my_reg.out

```

## Control Operators and Programs

A component's control program is defined by augmenting the list `my_component.control`. Control programs are constructed with [control operators][ctl].

### Group Enables

To [enable a group][en], include it in a component's control program.

```python
my_component.control += my_group

# using `get_group`
my_component.control += my_component.get_group("my_group")
```

### `seq`

Control statements are [sequenced][seq] in the order that they appear in a component's control program, represented by a list. Let's say we want to sequence the control statements `A`, `B`, and `C`.

```python
my_component.control += [A, B, C]
```

### `par`

For [parallel compositions][par] of control programs, use the `par()` method. Here's how to compose control programs `A` and `B` in parallel, and then sequence their composition with the control program `C`.

```python
my_component.control += [par(A, B), C]
```

### `if`

See the language reference for [`if`][if].

```python
# `if_(port, cond, body, else_body=None)`
my_if = if_(my_port, my_comb_group, my_true_group)

# with a nested if
my_other_if = if_(my_port, my_if)

# with a comb group to compute the value of my_port
my_if_comb = if_(my_port, my_comb_group, my_true_group)

# with an else body
my_if_else = if_(my_port, my_comb_group, my_true_group, my_false_group)

my_component.control += [my_if, my_other_if, my_if_comb, my_if_else]
```

### `while`

See the language reference for [`while`][while].

```python
# while_(port, cond, body)
my_while = while_(my_port, my_body)

# with a comb group to compute the value of my_port
my_while = while_(my_port, my_comb_group, my_body)
```

### `invoke`

See the language reference for [`invoke`][invoke].

```python
# invoke(cell, **kwargs)
my_invoke = invoke(my_cell, in_arg1=my_cell_arg1_reg.out, in_arg2=my_cell_arg2_reg.out)
```

## Miscellaneous Tips + Tricks

### Creating Handles

Handles allow components, groups, cells, control operators, and input/output ports to be referenced after their definition.

```python
def add_my_component(prog):
    # Creating a handle to a component
    my_component = prog.component("my_component")

    # using the component handle
    my_component.reg("my_reg", 32)

    # Creating a handle to an input/output port
    my_input = component.input("my_input", 32)
    my_output = component.output("my_output", 32)

    # Creating a handle to a cell
    my_second_comp = my_component.cell("my_cell", my_second_comp)

    # Creating a handle to a group
    with my_component.group("my_group") as my_group:
        # assignments

    # Creating a handle to a control operator
    my_if = if_(my_second_comp.out_port, body=my_group)

    # using the group handle + control operator handle
    my_component.control += [my_group, my_if]
```

### Importing Calyx Libraries

To generate imports for Calyx libraries, use the `Builder.import_()` method.

```python
prog = cb.Builder()
prog.import_("primitives/binary_operators.futil")
```

### Explictly Stating Widths with `const`

Usually, the builder library can automatically infer the width of a port. In cases where it can't, use the `const(width, value)` expression:

```python
my_cell.my_port = const(32, 1)
```

### High and Low Signals

The `calyx.builder.HI` or `calyx.builder.LO` are shorthand for one-bit high and low signals.

```python
"""A one-bit low signal"""
LO = const(1, 0)
"""A one-bit high signal"""
HI = const(1, 1)
```

[comb]: ../lang/ref.md#comb-group-definitions
[cont]: ../lang/ref.md#continuous-assignments
[ctl]: ../lang/ref.md#the-control-operators
[en]: ../lang/ref.md#group-enable
[ext]: ../lang/data-format.md#external-memories
[grp]: ../lang/ref.md#group-definitions
[hndl]: ref.md#creating-handles
[if]: ../lang/ref.md#if
[invoke]: ../lang/ref.md#invoke
[multi]: ../lang/multi-component.md
[par]: ../lang/ref.md#par
[ref]: ../lang/memories-by-reference.md#passing-cells-by-reference
[seq]: ../lang/ref.md#seq
[static]: ../lang/static.md#delay-by-n-cycles
[top]: ref.md#top-level-program-structure
[while]: ../lang/ref.md#while
