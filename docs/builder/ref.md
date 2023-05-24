# Builder Library Reference

## Top-Level Program Structure

Here's the general structure of a program that uses the builder to generate Calyx code.

```python
# import the builder library
import calyx.builder as cb


# define my_subcomponent
def add_my_subcomponent(prog):
    # subcomponent definition here...


# define my_component
def my_component(prog, my_subcomponent): 
    # add the component to the program
    my_component = prog.component("my_component")

    # add my_subcomponent as a cell of my_component
    my_subcomponent = my_component.cell("my_subcomponent", my_subcomponent)

    # define a my_component group
    with my_component.group("my_group") as my_group:
      # group assignments here ...

  my_component.control += [my_group]


# assemble the program
def build():
    prog = cb.Builder()
    my_subcomponent = add_my_subcomponent(prog)
    add_main(prog, my_subcomponent)
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

Note that you can create a handle to the component (i.e., write `my_component = prog.component("my_component")`) if you'd like to use that component by name.

### Retrieving Components

If you didn't [store a handle](#defining-components) to your component when you initialized it, you can do so later with the `Builder().get_component()` method.

```python
prog = cb.Builder()
prog.component("my_component")
# a few lines later ...
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
      # some other assignments...
        this.out = sum.out
```

### Adding Cells to a Component

## Groups

### Defining Groups

### Retrieving Groups

If you didn't [store a handle](#defining-components) to your group when you initialized it, you can do so later with the `Builder().get_group()` method.

```python
prog = cb.Builder()
my_component = prog.component("my_component")

with my_component.group("my_group"):
    # group definition here...

# a few lines later...
my_group = prog.get_group("my_group")
```

## Control

A component's control program is defined by augmenting the list `my_component.control`.

### Sequencing Groups

Groups are sequenced in the order that they appear in a component's control program list. Let's say we want to sequence groups `A`, `B`, and `C`.

```python
my_component.control += [A, B, C]
```

## Miscellaneous Tips + Tricks

### Importing Calyx Libraries

You can generate imports for Calyx libraries with the `Builder.import_()` method.

```python
prog = cb.Builder()
prog.import_("primitives/binary_operators.futil")
```
