# Calyx Language Tutorial

This tutorial will familiarize you with the Calyx language by writing a minimal program *by hand*.
Of course, the usual thing to do is to generate Calyx code from a DSL compiler frontend instead, but by writing a program manually, you can get familiar with all the basics in the language that you will need to do fancier things.


## Get Started

We will start with this skeleton, which just imports the standard library and defines an empty *component*, called `main`:

    import "primitives/std.lib";

    component main() -> () {
      cells {
      }

      wires {
      }

      control {
      }
    }

Put this in a file—you can call it `hello.futil`, for example.
(The `futil` file extension comes from an old name for Calyx.)

You can think of a component as a unit of Calyx code roughly analogous to a function: it encapsulates a logical unit of hardware structures along with their control.
Every component definition has three sections:

* `cells`: The hardware subcomponents that make up this component.
* `wires`: A set of guarded connections between components, possibly organized into *groups*.
* `control`: The imperative program that defines the component's execution schedule: i.e., when each group executes.

We'll fill these sections up minimally in the next sections.


## A Memory Cell and Input Data

Let's turn out skeleton into a tiny, nearly no-op Calyx program.
We'll start by adding a memory component to the cells:

    cells {
        mem = prim std_mem_d1(32, 1, 1);
    }

This new line declares a new cell called `mem`.
The `prim` keyword means that we're instantiating a primitive component: here, `std_mem_d1`, our standard-library component that represents a 1D memory.
You can see the definition of `std_mem_d1`, and all the other standard components, in the `primitives/std.lib` library we imported.
This one has three parameters:
the data width (here, 32 bits),
the number of elements (just one),
and the width of the address port (one bit).

We can almost run this program!
But first, we need to provide it with data.
The Calyx infrastructure can provide data to programs from [JSON][] files.
So make a file called something like `hello.json` containing something along these lines:

    {
      "mem": {
        "data": [10],
        "bitwidth": 32
      }
    }

The `mem` key means we're providing the initial value for our memory called `mem`.
We have one (integer) data element, and we indicate the width (32 bits).


## Compile & Run

If you want to see how this Calyx program compiles to Verilog, here's the fud incantation you need:

    fud exec hello.futil --to verilog

Not terribly interesting!
However, one nice thing you can do with programs is execute them.

Here's the fud incantation to run our program using [Verilator][]:

    fud exec hello.futil --to dat -s verilog.data hello.json

Using `--to dat` asks fud to run the program, and the extra `-s verilog.data <filename>` argument tells it where to find the input data.

[json]: https://www.json.org/
[verilator]: https://www.veripool.org/wiki/verilator
