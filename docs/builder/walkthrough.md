# Builder Library Walkthrough

This is an extended walkthough of all the features of the Calyx builder library. The builder library is an embedded DSL, embedded in Python, that allows users to generate Calyx code programmatically.

This page seeks to demonstrate all the features of the builder library. For a quick start, we refer you to the [hello world example][helloworld].

We will make repeated references to the example program [`walkthrough.py`][walkthrough], which emits the Calyx code[`walkthrough.expect`][walkthrough_expect] when run. We recommend that you refer to these files as you work through this document.

## Components

We add a component to our program as follows:

```python
{{#include ../../calyx-py/test/walkthrough.py:component}}
```

## Ports of Components

We specify the names and bitwidths of any ports that we want a component to have as follows:

```python
{{#include ../../calyx-py/test/walkthrough.py:ports}}
```
Observe that we have saved handles to the input ports by assigning them to Python variables, but have not done the same with the output port.
We will show shortly how to create a handle to a port after its definition.

## Cells

We add cells to the component as follows.
The standard cells are all supported.
Bitwidths must be passed as arguments, while names are optional.

```python
{{#include ../../calyx-py/test/walkthrough.py:cells}}
```
The adder defined above is unsigned; we would define a signed variant as:
```python
    add = comp.add(32, signed=True)
```

## Groups

We begin a group with:

```python
{{#include ../../calyx-py/test/walkthrough.py:group_def}}
```

We add wires within a group by staying within the indentation of the `with` block.

[Combinational groups][comb] are written similarly, but with `comb_group` instead of `group`:

```python
    with comp.comb_group("compute_sum") as compute_sum:

```

## Ports of Cells

We access ports of cells using dot notation.

```python
{{#include ../../calyx-py/test/walkthrough.py:dot_notation}}
```

### Special Case: `in_`

We specify the value to be written to a register with:
```python
{{#include ../../calyx-py/test/walkthrough.py:in_}}
```
Although the Calyx port is named `in`, we must write `in_` in the eDSL to avoid a clash with Python's `in` keyword.

## HI and LO Signals

The builder library provides shorthand for high and low signals.

```python
{{#include ../../calyx-py/test/walkthrough.py:high_signal}}
```
There is a corresponding `LO` signal.
These are just one-bit values `1` and `0`, respectively.



## Group `done` Signals

Groups that are not [combinational][comb] must raise a `done` signal.

```python
{{#include ../../calyx-py/test/walkthrough.py:done}}
```

## Accessing Output Ports of Components

We can create a handle to a port after its definition.

```python
{{#include ../../calyx-py/test/walkthrough.py:this_continuous}}
```
That is, `comp.this().out` is a handle to the port named "out" on the component whose handle is `comp`.

Accessing ports in this way may feel silly, since we have already shown that we can save handles to ports by assigning them to Python variables. This does work for input ports, but not for output ports.

Say we had saved a handle to the output port of the adder component:
```
out = comp.output("out", 32)
```

Now say we wanted to say that the output port gets the value of the sum's output port:
```python
out = sum.out
```

Python will get in our way because it will think that `out` is a variable that is written to (twice!) but never read from.

To avoid this, we use the `this()` method to access the output ports of a component.

## Continuous Assignments
[Continuous assignments][cont] are added using `with {component}.continuous:`.

```python
{{#include ../../calyx-py/test/walkthrough.py:this_continuous}}
```

## Simple Control Program

A simple control program is added to the component as follows:

```python
{{#include ../../calyx-py/test/walkthrough.py:control}}
```

## Binary Operation and Store

The library provides a shorthand for the common pattern of performing a binary operation and writing the result to a register.

```python
{{#include ../../calyx-py/test/walkthrough.py:sub_and_store}}
```

Here `diff` is a handle to a register that we have defined earlier. This single line of Python adds lines to the `cells` and the `wires` sections of the Calyx code:
```
  cells {
    sub_1 = std_sub(32);
  }
  wires {
    group sub_1_group {
      sub_1.left = val1;
      sub_1.right = val2;
      diff.write_en = 1'd1;
      diff.in = sub_1.out;
      sub_1_group[done] = diff.done;
    }
  }
```
In Python, its return value is a handle to the group that it has created, and a handle to the register is has written to. In the line of Python above, we have saved the handle to the group (as `diff_group_1`) but have discarded the handle to the register using a `_` variable name since we already have a handle to the register, `diff`.

This construct can also be called without passing a register, in which case it will create a register and return it. It is useful in that case to save the handle to the register.

## Operation-Use

The library provides a shorthand for the common pattern of performing a binary operation and using the result combinationally.

```python
{{#include ../../calyx-py/test/walkthrough.py:lt_use_oneliner}}
```

This line of Python adds lines to the `cells` and the `wires` sections of the Calyx code:
```
cells {
    lt_3 = std_lt(32);
  }
  wires {
    comb group lt_3_group {
      lt_3.left = val2;
      lt_3.right = val1;
    }
  }
```

Note that the group is combinational, and so does not need a `done` signal.

The value returned by this function, which we have saved above as `val2_lt_val1` is in fact a tuple of handles: a handle to the group that it has created and a handle to the cell that that group uses. We shall see shortly how to use this tuple.

## Complex Control: `par`, `seq`, `if`

Let us work through a slightly more complex control program.

```python
{{#include ../../calyx-py/test/walkthrough.py:par_if_ifwith}}
```

We run control operators in sequence by making them elements of a list. This is why the group `val1_ge_val2` runs before the if check written on the next line.

We run control operators in parallel by passing them to the `par` function.

The `if_` function (named with the underscore to avoid clashing with Python's `if` keyword) is a straightforward `if` check. It takes a condition, a body, and an optional else body.

The `if_with` function is a slightly more complex `if` check. It takes a (`cell`, `comb_group`) tuple, a body, and an optional else body.
It generates a combinational if check, of the form
```
if cell.out with comb_group ...
```
It is especially useful in concert with the `Operation-Use` construct, which returns exactly such a tuple of a cell and a group.

## Multi-Component Designs

Using one component in another is straightforward.

We must first define the called component as a cell of the calling component, and then we can use the cell as usual.

Say we have a handle, `diff_comp`, to the component that we wish to call. Say also that we know that the component has input ports `val1` and `val2`, and an output port `out`.

We can write:

```python
{{#include ../../calyx-py/test/walkthrough.py:multi-component}}
```
Although the called component did not have explicit `go` and `done` ports, the builder library has added them for us.
We use these ports to guide the execution of the group.
We assert the `go` signal to the called component with `diff_comp.go = HI`, and then, by writing `mux.write_en = abs_diff.done`, we make the write to the register `mux` conditional on the `done` signal of the called component.


## Memories

We can define memories in a component as follows:

```python
{{#include ../../calyx-py/test/walkthrough.py:comb_mem_d1_ref}}
```

This is a 1-D memory with ten 32-bit entries, each 32 bits wide. We have additionally declared that this memory will be passed to the component by reference. We shall see how shortly.


## Miscellaneous Higher-Level Constructors

As patterns of use emerge, we can add further constructors to the builder library to support common use-cases.
For example, we can add a constructor that increments a register by a constant value.

```python
{{#include ../../calyx-py/test/walkthrough.py:incr_oneliner}}
```

That line of Python adds lines to the `cells` and the `wires` sections of the Calyx code:
```
  cells {
    i_incr = std_add(8);
  }
  wires {
    group i_incr_group {
      i_incr.left = i.out;
      i_incr.right = 8'd1;
      i.write_en = 1'd1;
      i.in = i_incr.out;
      i_incr_group[done] = i.done;
    }
  }
```

The Python return value, `incr_i`, is a handle to the group that performs the incrementing. The method defaults to incrementing by 1, but can be passed any value.

## Guarded Assignments

Consider the group that adds value `v` to a memory at the cell pointed to by register `i`.

```python
{{#include ../../calyx-py/test/walkthrough.py:add_at_position_i}}
```

The first few lines are straightforward; we are setting the cell to be read from with `addr0`, reading from that cell and driving the value to the adder's left port, and setting the right port of the adder to the value `v`.

Now we wish to write the result to the memory at the cell pointed to by register `i`, but only once we know that the adder has finished its work. We do this with a guarded assignment, using the `@` operator:
```python
        mem.write_en = add.done @ cb.HI
```
In Calyx, we would have written this guarded assignment with a question mark:
```
        mem.write_en = add.done ? 1'd1;
```
We use the `@` operator in the builder library to avoid clashing with Python's ternary operator.

## Complex Control: `while`

The builder library supports `while` loops and also the higher-level `while_with` constructor.

```python
{{#include ../../calyx-py/test/walkthrough.py:while_with}}
```

Here `i_lt_10` is a tuple of two handles, exactly as returned by the `Operation-Use` constructor. The `while_with` constructor takes this tuple and a body.

## External Memories

We can define external memories in a component, typically the `main` component, as follows:

```python
{{#include ../../calyx-py/test/walkthrough.py:ext_mem}}
```

## Invoking Components

We can invoke components as follows:

```python
{{#include ../../calyx-py/test/walkthrough.py:invoke}}
```

That is, we have a Python-level handle to some component `map` that has one memory called `mem` that it expects to be passed by reference, and one input port called `v`.
We must prepend `ref_` to the names of any memories, and `in_` to the names of any input ports.

## Building the Program

Finally, we build the program.

```python
{{#include ../../calyx-py/test/walkthrough.py:build}}
```

Note that all of our component-inserting helpers have been _returning_ the components that they have created. This is so that we can build complex programs where components either call each other as cells or invoke each other.

This is why we save a Python-level handle to the `diff_comp` component that we have defined, and then pass it to the `insert_mux_component` function. As we have seen, the `mux` uses `diff_comp` as a cell.

We also save a Python-level handle to the `map` component that we have defined, and then pass it to the `insert_main_component` function. As we have seen, `map` is invoked by `main`.

## Emitting the Program

Finally, we emit Calyx.

```python
{{#include ../../calyx-py/test/walkthrough.py:emit}}
```

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

### Defining Component Attributes

Components can be given attributes. Similar to ports, just specify the name of the attribute and its value.
Note that `attribute(name, value)` does not return a handle to the attribute.

```python
my_component.attribute("my_attribute", 1)
```

Will create a component that looks like:

```
component my_component<"my_attribute"=1>(...) -> (...) {
```

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

Control statements are [sequenced][seq] in the order that they appear in a component's control program, represented by a Python list. Let's say we want to sequence the control statements `A`, `B`, and `C`.

```python
my_component.control += [A, B, C]
```

### `par`

For [parallel compositions][par] of control programs, use the `par()` function. Here's how to compose control programs `A` and `B` in parallel, and then sequence their composition with the control program `C`.

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
my_invoke = invoke(my_cell, in_arg1=my_cell_arg1_reg.out, in_arg2=1)

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
[helloworld]: helloworld.md
[walkthrough]: https://github.com/calyxir/calyx/blob/master/calyx-py/test/walkthrough.py
[walkthrough_expect]: https://github.com/calyxir/calyx/blob/master/calyx-py/test/walkthrough.expect