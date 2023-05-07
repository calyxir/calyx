# MrXL

> The MrXL frontend is a toy frontend developed for the [frontend tutorial][fronttut].

MrXL is an example DSL for demonstrating Calyx. MrXL programs consist of `map` and `reduce` operations on arrays. For example, this is a dot product implementation:

    input avec: int[1024]
    input bvec: int[1024]
    output dot: int
    prodvec := map 16 (a <- avec, b <- bvec) { a * b }
    dot := reduce 4 (a, b <- prodvec) 0 { a + b }

The numbers that come right after `map` and `reduce` (16 and 4 respectively) are "parallelism factors" that guide the generation of hardware.


Install
-------

Install the [calyx-py](../calyx-py.md) library.

The MrXL implementation is in Python and uses [Flit][].
First, [install flit][flit] (`pip install flit` or similar), and then type the
following inside `frontend/mrxl`:

    flit install --symlink

This creates a symbolic link the mrxl directory and installs the `mrxl` command
line tool.

By default, [fud](../fud) looks for the `mrxl` executable to enable
the `mrxl` compilation stage.
Type `fud check` to make sure `fud` reports that the `mrxl` compiler has been
found.


Interpreter
-----------

To run the interpreter, run:

    mrxl <program> --data <indata> --interpret

where `<program>` is a MrXL source code file and `<indata>` is a JSON file containing values for all the variables declared as `input`s in the program. The interpreter dumps the `output` variables as JSON to stdout.

You could try, for example:

    mrxl test/dot.mrxl --data test/dot.mrxl.data --interpret

We also provide `add.mrxl` and `sum.mrxl`, along with sample `<indata>` files, under `test/`. Try playing with the inputs and the operations!

Compiling to Calyx
------------------

To run the compiler, and see the Calyx code your MrXL program generates, just leave off the `--data` and `--interpret` flags. For instance:

    mrxl test/dot.mrxl

In order to run the compiler through `fud`, pass the `--from mrxl` flag:

    fud e --from mrxl <mrxl file> --to futil

To simulate the Verilog generated from the mrxl compiler, set the `-s
verilog.data` as usual:

    fud e --from mrxl <mrxl file> --to dat -s verilog.data <data file>


[flit]: https://flit.readthedocs.io/en/latest/index.html
[fronttut]: ../tutorial/frontend-tut.md
