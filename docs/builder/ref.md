# Builder Library Reference

## Top-Level Program Structure

Here's the general structure of a program that uses the builder to generate Calyx code.

```python
# import the builder library
import calyx.builder as cb


# define my_subcomponent
def add_my_subcomponent(prog):
    # subcomponent definition here


# define my_component
def my_component(prog, my_subcomponent): 
    # add the component to the program
    my_component = prog.component("my_component")

    # add my_subcomponent as a cell of my_component
    my_subcomponent = my_component.cell("my_subcomponent", my_subcomponent)

    # define a my_component group
    with my_component.group("my_group") as my_group:
      # group assignments here 

  my_component.control += [my_group]


# assemble the program
def build():
    prog = cb.Builder()
    my_subcomponent = add_my_subcomponent(prog)
    add_main(prog, my_subcomponent)

    # return the generated program
    return prog.program


# emit the program
if __name__ == "__main__":
    build().emit()
```

## Components

### Defining Components

You can define a component by calling the `Builder().component()` method.

```python
prog = cb.Builder()
prog.component("my_component")
```

Components can also be instantiated with a list of cells.

```python

 adders = [adder_1, adder_2, adder_3, adder_4] = [
        my_component.add(name, 32) for name in ["1", "2", "3", "4"]
    ]
my_component = prog.component("my_component", cells=adders)
```

### Retrieving Components

To reference a component you did not [store a handle][hndl], you can do so later with the `Builder().get_component()` method.

```python
prog = cb.Builder()
prog.component("my_component")
# a few lines later 
my_component = prog.get_component("my_component")
```

### Defining component inputs and outputs

Components can be given input and output ports. All you have to do is specify the name of the port and its size.

```python
my_component.input("my_input", 32)
my_component.output("my_output", 32)
```

You can access the input and output ports of a component within the definition of a component using the standard `this.port` syntax.

```python
def add_my_component(prog):
    my_component = prog.component("my_component")

    this = my_component.this()
    with my_component.group("my_group")
      # some other assignments
        this.out = sum.out
```

### Component Cells

### Multi-Component Designs

Calyx supports [multi-component designs][multi]. The [top-level example](ref.md#top-level-program-structure) demonstrates how to construct multi-component designs using the library.

#### Defining Common Calyx Cells

Here's a snippet of code that adds a few common kinds of cells to a component:

```python
my_component = prog.component("my_component")

# Registers: reg(name, bitwidth)
my_component.reg("my_reg", 32)

# Constants: const(name, bitwidth, value)
my_component.const("my_reg", 32, 42)

# Adders: 
my_component.add()

# Subtractors: 

# Comparators: [gt|lt|eq|neq|ge|le](name, size, signed=False)
my_component.gt("my_gt", 32)
# A signed lt comparison
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

## Wires

### Groups

#### Defining Groups

[Groups][grp] are defined using the `group()` method for a component.

```python
my_component = prog.component("my_component")

with my_component.group("my_group") as my_group:
    # group assignments here
```

#### Defining Combinational Groups

[Combinational groups][comb] are defined similarly to groups, except with the `comb_group` method.

```python
my_component = prog.component("my_component")

with my_component.comb_group("my_comb_group"):
    # group assignments here
```

#### Retrieving Groups

If you didn't [store a handle](ref.md#creating-handles-to-components) to your group when you initialized it, you can do so later with the `Builder().get_group()` method.

```python
prog = cb.Builder()
my_component = prog.component("my_component")

with my_component.group("my_group"):
    # group definition here

# a few lines later
my_group = prog.get_group("my_group")
```

### Continuous Assignments

### Guarded Assignments

## Control Operators and Programs

A component's control program is defined by augmenting the list `my_component.control`. Control programs are constructed with [control operators][ctl].

### `seq`

Control statements are sequenced in the order that they appear in a component's control program, represented by a list. Let's say we want to sequence the control statements `A`, `B`, and `C`.

```python
my_component.control += [A, B, C]
```

### `par`

For parallel compositions of control programs, use the `par()` method. Here's how to compose control programs `A` and `B` in parallel, and then sequence their composition with the control program `C`.

```python
my_component.control += [par(A, B), C]
```

### `if`

```python
# if_(port: ExprBuilder, cond, body, else_body=None)
my_component.control += [if_(my_port, )]
```

### `while`

### `invoke`

## Miscellaneous Tips + Tricks

### Creating Handles

You can create handles to components, groups, and cells if you'd like to use them by name later on.

```python
# Creating a handle to a component
my_component = prog.component("my_component")

# using the handle
my_component.input("my_in", 32)

# Creating a handle to a group
def add_my_component(prog):
    my_component = prog.component("main")

    with my_component.group("my_group") as my_group:
        # assignments

    # using the handle
    my_component.control += [my_group]

# Creating a handle to a cell
my_subcomponent = my_component.cell("my_cell", my_subcomponent)
```

### Accessing Ports

### External Memories and `.data` Files

### Importing Calyx Libraries

You can generate imports for Calyx libraries with the `Builder.import_()` method.

```python
prog = cb.Builder()
prog.import_("primitives/binary_operators.futil")
```

### Explictly stating widths with `const`

Usually, the builder library can automatically infer the width of a port. In cases where it can't, you can use the `const(width, value)` expression:

```python
my_cell.my_port = const(32, 1)
```

### High and Low Signals

You can use `calyx.builder.HI` or `calyx.builder.LO` as shorthand for one-bit high and low signals.

```python
"""A one-bit low signal"""
LO = const(1, 0)
"""A one-bit high signal"""
HI = const(1, 1)
```

[comb]: ../lang/ref.md#comb-group-definitions
[ext]: ../lang/data-format.md#external-memories
[ref]: ../lang/memories-by-reference.md#passing-cells-by-reference
[multi]: ../lang/multi-component.md
[ctl]: ../lang/ref.md#the-control-operators
[grp]: ../lang/ref.md#group-definitions
[hndl]: ref.md#creating-handles-to-components
