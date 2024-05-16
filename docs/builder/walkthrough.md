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
    with comp.comb_group("update_register") as update_register:

```

Static groups are written with `static_group`, and must specify a latency. The group below will take 3 cycles to execute:

```python
    with comp.static_group("multiply", 3) as compute_sum:
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

## `HI` and `LO` Signals

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

A simple control program is added to the component as follows.
This is just [enabling][en] the group that we have defined.

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

We run control operators in [sequence][seq] by making them elements of a list. This is why the group `val1_ge_val2` runs before the if check written on the next line.

We run control operators in [parallel][par] by passing them to the `par` function.

The `if_` function (named with the underscore to avoid clashing with Python's `if` keyword) is a straightforward [if][] check. It takes a condition, a body, and an optional else body.

The `if_with` function is a slightly more complex [if][] check. It takes a (`cell`, `comb_group`) tuple, a body, and an optional else body.
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

This is a 1-D memory with ten 32-bit entries, each 32 bits wide. We have additionally declared that this memory will be passed to the component [by reference][ref]. We shall see how shortly.

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

The builder library supports [while][] loops and also the higher-level `while_with` constructor.

```python
{{#include ../../calyx-py/test/walkthrough.py:while_with}}
```

Here `i_lt_10` is a tuple of two handles, exactly as returned by the `Operation-Use` constructor. The `while_with` constructor takes this tuple and a body.

## External Memories

We can define [external memories][ext] in a component, typically the `main` component, as follows:

```python
{{#include ../../calyx-py/test/walkthrough.py:ext_mem}}
```

## Invoking Components

We can [invoke] components as follows:

```python
{{#include ../../calyx-py/test/walkthrough.py:invoke}}
```

That is, we have a Python-level handle to some component `map` that has one memory called `mem` that it expects to be passed [by reference][ref], and one input port called `v`.
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

## Retrieving Items by Name

In the discussion so far, we have guided you towards a pattern of defining an item (a cell, a group, a component, etc.) and then saving a handle to it as a Python variable. This is a good pattern to follow, but it is not the only one.

To reference a component without an existing handle to it, use the `Builder().get_component()` method.

```python
prog.component("my_component")
# a few lines later
my_component = prog.get_component("my_component")
```

To access the input and output ports of a component within the definition of a component, use the syntax `my_component.this().port`.

```python
def add_my_component(prog):
    my_component = prog.component("my_component")
    my_component.output("my_output", 32)

    with my_component.group("my_group"):
        my_component.this().my_output = const(32, 1)
```

In order to reference a cell without a handle use the `Builder().get_cell()` method.

```python
my_component.reg("my_reg", 32)
# a few lines later
my_reg = prog.get_cell("my_reg")
```

A group can be retrieved with the `Builder().get_group()` method. It's possible to retrieve combinational groups as well as regular groups with this method.

```python
with my_component.group("my_group"):
    # group definition here
# a few lines later
my_group = prog.get_group("my_group")

```

## Defining Component Attributes

Components can be given attributes. Similar to ports, just specify the name of the attribute and its value.
Note that `attribute(name, value)` does not return a handle to the attribute.

```python
my_component.attribute("my_attribute", 1)
```

Will create a component that looks like:

```
component my_component<"my_attribute"=1>(...) -> (...) {
```

## Importing Calyx Libraries

The builder library imports necessary Calyx libraries automatically. However, it is possible to import additional libraries manually.

```python
prog = cb.Builder()
prog.import_("primitives/binary_operators.futil")
```

## Explictly Stating Widths

Usually, the builder library can automatically infer the widths of constants. In cases where it cannot, it will complain at Python compilation. Use the `const(width, value)` expression to explicitly state the width of a constant.

```python
my_cell.my_port = const(32, 1)
```

## Components with Known Latency

You can declare a component to be `static` by stating its latency when declaring it.
For instance, our contrived adder from above could be declared as static with a latency of one cycle as follows:

```python
comp = prog.component("adder", latency=1)
```

As a reminder,  the regular version is just:

```python
comp = prog.component("adder")
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
[helloworld]: calyx-py.md
[walkthrough]: https://github.com/calyxir/calyx/blob/master/calyx-py/test/walkthrough.py
[walkthrough_expect]: https://github.com/calyxir/calyx/blob/master/calyx-py/test/walkthrough.expect