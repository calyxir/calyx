# Emitting Calyx from Python

The `calyx` builder library can be used to generate Calyx code in Python.
## Installation

To install the library, run the following from the repository root (requires
[flit][] installation):

```
cd calyx-py && flit install -s
```

## Using the `calyx` builder

The `calyx` library provides a builder to generate Calyx code. The [library reference][ref] documents most builder methods and constructs.

We will also walk through the file [`builder_example.py`][example] to demonstrate how the builder library is used. This Calyx program initializes two registers with the numbers 1 and 41, adds them together, and stores the result in a register.

The `add_main_component(prog)` method will, as the name suggests, add a main component to our program. We can define components for our Calyx program `prog` with `prog.component`. Here's a defininition of a `main` component with a 32-bit input `in` and output `out`:

```python
{{#include ../../calyx-py/test/builder_example.py:init}}
```

Technically, we didn't need to assign `prog.component("main")` to a variable; the component `main` would have been added to `prog` regardless. However, it will often prove useful to store handles to components, registers, or other objects you'd like to use later.

We then instantiate our cells: three 32-bit registers and one 32-bit adder.

```python
{{#include ../../calyx-py/test/builder_example.py:cells}}
```

As with adding components to a program, we don't need to assign `main.reg(...)` to a variable, but it'll be useful to be able to quickly refer to these cells.

Next, we'll define our groups of assignments. The syntax for defining a group looks like `with {component}.group("group_name") as group_variable`, as we do below:

```python
{{#include ../../calyx-py/test/builder_example.py:group_def}}
```

Now, we'll initialize our registers. You can access cell ports using dot notation. Notably, port names that are also reserved keywords in Python such as `in` are followed by an underscore.

```python
{{#include ../../calyx-py/test/builder_example.py:assigns}}
```

As mentioned in the comments above, the Calyx builder will try to infer the bitwidth of constants. In case this doesn't work and you run into problems with this, you can provide the constant's size like so:

```python
{{#include ../../calyx-py/test/builder_example.py:const}}
```

Calyx groups use a [latency-insensitive go/done interface][godone]. When the `done` signal of a component is `1`, it signals that the component has finished executing. Oftentimes, computing this signal is conditional. We use [guarded assignements][guarded] to a group's done signal in order to express this. Writing a group's done condition with the builder is pretty similar to doing so in Calyx, except that the `?` used for guarded assignments is now `@` (due to conflicting usage of `?` in Python).

```python
{{#include ../../calyx-py/test/builder_example.py:done}}
```

In order to use the ports of cells in our `main` component within the code for our component, we'll expose the adder's output port by explicitly constructing it using the `calyx-py` AST.

```python
{{#include ../../calyx-py/test/builder_example.py:bare}}
```

Now, when we want to use the output port of our adder, we can do so easily:

```python
{{#include ../../calyx-py/test/builder_example.py:bare_use}}
```

In order to add [continuous assignments][cont] to your program, use the construct `with {component}.continuous:`.

```python
{{#include ../../calyx-py/test/builder_example.py:continuous}}
```

To access a component's ports while defining it, like we did above, we use the method `this()`.

```python
{{#include ../../calyx-py/test/builder_example.py:this}}
```

Lastly, we'll construct the control portion of this Calyx program. It's pretty simple; we're running two groups in sequence. Sequences of groups are just Python lists:

```python
{{#include ../../calyx-py/test/builder_example.py:control}}
```

You can also use the builder to generate parallel control blocks. To do this, use the `par` keyword. For instance, the above code with some parallel groups in it might look like

```python
    main.control += [
        update_operands,
        compute_sum,
        par(A, B, C)
    ]
```

After making our modifications to the `main` component, we'll build the program using the `build()` method. We use the `Builder` object to construct `prog`, and then return the generated program.

```python
{{#include ../../calyx-py/test/builder_example.py:return}}
```

Finally, we can emit the program we built.

```python
{{#include ../../calyx-py/test/builder_example.py:emit}}
```

That's about it for getting started with the `calyx-py` builder library! You can inspect the generated Calyx code yourself by running:

```python
python calyx-py/test/builder_example.py
```

Other examples using the builder can also be found in the `calyx-py` [test directory][test]. All of our frontends were also written using this library, in case you'd like even more examples!

[cont]: ../lang/ref.md#continuous-assignments
[example]: https://github.com/cucapra/calyx/blob/master/calyx-py/test/builder_example.py
[flit]: https://flit.readthedocs.io/en/latest/
[godone]: ..lang/ref.md#the-go-done-interface
[guarded]: ../lang/ref.md#guarded-assignments
[ref]: ref.md
[test]: https://github.com/cucapra/calyx/tree/master/calyx-py/test/
