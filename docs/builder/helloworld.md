# Emitting Calyx from Python

The `calyx` builder library provisions an embedded domain-specific language (eDSL) that can be used to generate Calyx code.
The DSL is embedded in Python.

## Installation

To install the library, run the following from the repository root.
The command requires [flit][], which you can install with `pip install flit`.

```
cd calyx-py && flit install -s
```

## Hello, Calyx World!

We will start by using the `calyx` library to generate a simple Calyx program.
Glance through the Python code below, which is also available at [`helloworld.py`][helloworld].

```python
{{#include ../../calyx-py/test/helloworld.py}}
```
Running this Python code, with
```python
python calyx-py/test/helloworld.py
```
will generate the following Calyx code.
As you may have inferred, we are have simply created a 32-bit adder in a contrived manner.

```calyx
{{#include ../../calyx-py/test/helloworld.expect}}
```

## Walkthrough

So far, it does not look like using our eDSL has bought us much.
We have essentially written Calyx, line by line, in Python.
However, it is useful to go through the process of generating a simple program to understand the syntax and semantics of the builder library.

For each item discussed below, we encourage you to refer to both the Python code and the generated Calyx code.

We add the component `adder` to our program with the following line:

```python
{{#include ../../calyx-py/test/walkthrough.py:component}}
```

We then specify the names and bitwidths of any ports that we want the component to have.

```python
{{#include ../../calyx-py/test/walkthrough.py:ports}}
```

We also add two cells to the component: a 32-bit adder and a 32-bit register.

```python
{{#include ../../calyx-py/test/walkthrough.py:cells}}
```

The heart of the component is a group of assignments.
We begin the group with:

```python
{{#include ../../calyx-py/test/walkthrough.py:group_def}}
```

We know that we have instantiated a `std_add` cell, and that such a cell has input ports `left` and `right`.
We need to assign values to these ports, and we do so using straightforward dot-notated access.
The values `val1` and `val2` exactly the inputs of the component.

```python
{{#include ../../calyx-py/test/walkthrough.py:dot_notation}}
```

Now we would like to write the output of the adder to a register.
We know that registers have input ports `write_en` and `in`.
We assert the high signal on `write_en` with:
```python
{{#include ../../calyx-py/test/walkthrough.py:high_signal}}
```
We specify the value to be written to the register with:
```python
{{#include ../../calyx-py/test/walkthrough.py:in_}}
```
Although the port is named `in`, we must use `in_` to avoid a clash with Python's `in` keyword.
Observe that we have used dot-notated access to both _read_ the `out` port of the adder and _write_ to the `in` port of the register.

Since `compute_sum` is a group of assignments, we must specify when it is done. We do this with:

```python
{{#include ../../calyx-py/test/walkthrough.py:done}}
```
That is, the group is done when the register we are writing into asserts _its_ `done` signal.

In order to add a [continuous assignment][cont] to our program, we use the construct `with {component}.continuous:`.
To access the ports of a component while defining it, we use the `this()` method.

```python
{{#include ../../calyx-py/test/walkthrough.py:this_continuous}}
```
That is, we want this component's `out` port to be continuously assigned the value of the `sum`'s `out` port.

Finally, we construct the control portion of this Calyx program:

```python
{{#include ../../calyx-py/test/walkthrough.py:control}}
```

We have some boilerplate code that creates an instance of the builder, adds to it the component that we have just studied, and emits Calyx code.
```python
if __name__ == "__main__":
    prog = cb.Builder()
    insert_adder_component(prog)
    prog.program.emit()
```
Further, the builder library is able to infer which Calyx libraries are needed in order to support the generated Calyx code, and adds the necessary `import` directives to the generated code.

## Further Reading

The [builder library walkthrough][walkthrough] contains a detailed description of the constructs available in the builder library.

Other examples using the builder can also be found in the `calyx-py` [test directory][test]. All of our frontends were also written using this library, in case you'd like even more examples!

[cont]: ../lang/ref.md#continuous-assignments
[flit]: https://flit.readthedocs.io/en/latest/
[godone]: ..lang/ref.md#the-go-done-interface
[guarded]: ../lang/ref.md#guarded-assignments
[walkthrough]: walkthrough.md
[test]: https://github.com/calyxir/calyx/tree/master/calyx-py/test/
[helloworld]: https://github.com/calyxir/calyx/blob/master/calyx-py/test/helloworld.py
