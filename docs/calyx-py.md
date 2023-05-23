# Emitting Calyx from Python

Our frontends are written in Python3 and make use of the `calyx` builder library to
generate their code.

## Installation

To install the library, run the following from the repository root (requires
[flit][] installation):

```
cd calyx-py && flit install -s
```

## Using the `calyx` builder

The `calyx` library provides a builder to generate Calyx code. Let's walk through the file `calyx-py/test/builder_example.py`. This Calyx program initializes two registers with the numbers 1 and 41, adds them together, and stores the result in a register.

We use the `Builder` object to construct our Calyx program `prog`. We can define components with `prog.component`. Here's a defininition of a `main` component with a 32-bit input and output:

```python
{{#include ../calyx-py/test/builder_example.py:init}}
```

Technically, we didn't need to assign `prog.component("main")` to a variable; the `main` component would have been added to `prog` regardless. However, it will often prove useful to store handles to components, registers, or other objects you'd like to use later.

We then instantiate our cells: three 32-bit registers and one 32-bit adder.

```python
{{#include ../calyx-py/test/builder_example.py:cells}}
```

As with adding components to a program, we don't need to assign `main.reg(...)` to a variable, but it'll be useful to be able to quickly refer to these cells.

Next, we'll define our groups of assignments. The syntax for defining a group looks like `with {component}.group("group_name") as group_variable`, as we do below:

```python
{{#include ../calyx-py/test/builder_example.py:group_def}}
```

Now, we'll initialize our registers. You can access cell ports using dot notation. Notably, port names that are also reserved keywords in Python such as `in` are followed by an underscore.

```python
{{#include ../calyx-py/test/builder_example.py:assigns}}
```

As mentioned in the comments above, the Calyx builder will try to infer the bitwidth of constants. In case this doesn't work and you run into problems with this, you can provide the constant's size like so:

```python
{{#include ../calyx-py/test/builder_example.py:const}}
```

Calyx groups use a latency-insensitive go/done interface. Writing a group's done condition with the builder is pretty similar to doing so in Calyx, except that the `?` used for guarded assignments is now `@` (due to conflicting usage of `?` in Python).

```python
{{#include ../calyx-py/test/builder_example.py:done}}
```

In order to use the ports of cells in our `main` component within the code for our component, we'll expose the adder's output port by explicitly constructing it using the `calyx-py` AST.

```python
{{#include ../calyx-py/test/builder_example.py:bare}}
```

Now, when we want to use the output port of our adder, we can do so easily:

```python
{{#include ../calyx-py/test/builder_example.py:bare_use}}
```

In order to add continuous assignments to your program, use the construct `with {component}.continuous:`.

```python
{{#include ../calyx-py/test/builder_example.py:continuous}}
```

To access a component's ports while defining it, like we did above, we use the method `this()`.

```python
{{#include ../calyx-py/test/builder_example.py:this}}
```

Lastly, we'll construct the control portion of this Calyx program. It's pretty simple; we're sequencing two groups. Sequences of groups are just Python lists:

```python
{{#include ../calyx-py/test/builder_example.py:return}}
```
You can also use the builder for generating parallel control blocks. Just encapuslate the parallel groups (say, `A`, `B`, and `C`) with the `par` keyword. For instance, the above code with some parallel groups in it might look like

```python
    main.control += [
        update_operands,
        compute_sum,
        par(A, B)
    ]
```

After making our modifications to the `main` component, we'll return the program.

```python
{{#include ../calyx-py/test/builder_example.py:return}}
```

Finally, we can emit the program we built.

```python
{{#include ../calyx-py/test/builder_example.py:emit}}
```

In this program.`build()` serves both as a way to define the `main` component as well as organize the Calyx program we generated. Since this program was so simple, we didn't need to factor out our component definitions. For Calyx programs with multiple components or more complex operations, factoring definitions or procedures into their own functions is useful.

That's about it for getting started with the `calyx-py` builder library! You can inspect the generated Calyx code yourself by running:

```python
python calyx-py/test/builder_example.py
```

Other examples using the builder can also be found in the `calyx-py/test/` directory.

[flit]: https://flit.readthedocs.io/en/latest/
