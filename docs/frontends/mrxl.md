# MrXL

MrXL is an example DSL developed for the [frontend tutorial][fronttut].
MrXL programs consist of `map` and `reduce` operations on arrays. 
For example, here is a dot product implementation:

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
following after changing your directory to `frontend/mrxl`:

    flit install --symlink

This creates a symbolic link to the present directory and installs the `mrxl` command line tool.

By default, [fud](../fud) looks for the `mrxl` executable to enable
the `mrxl` compilation stage.
Type `fud check` to make sure `fud` reports that the `mrxl` compiler has been
found. If it does not, run the following while still in `frontend/mrxl`.

    fud register mrxl -p fud/mrxl.py

Run `fud check` again to ensure that `fud` sees `mrxl`.


Interpreting MrXL
-----------------

To run the program through the MrXL interpreter, execute:

    mrxl <program> --data <indata> --interpret

where `<program>` is a MrXL source code file and `<indata>` is a JSON file containing values for all the variables declared as `input`s in the MrXL program. The interpreter dumps the `output` variables, in a JSON format, to stdout.

You could try, for example:

    mrxl test/dot.mrxl --data test/dot.mrxl.data --interpret

This is just a baby version of the dot produce implementation we showed at the very top.
We also provide `add.mrxl` and `sum.mrxl`, along with sample `<indata>` files, under `test/`. Try playing with the inputs and the operations!


Compiling to Calyx
------------------

To run the compiler and see the Calyx code your MrXL program generates, just leave off the `--data` and `--interpret` flags. For instance:

    mrxl test/dot.mrxl

In order to run the compiler through `fud`, pass the `--from mrxl` flag:

    fud e --from mrxl <mrxl file> --to futil

To simulate the Verilog generated from the mrxl compiler, set the `-s
verilog.data`:

    fud e --from mrxl <mrxl file> --to dat --through verilog -s verilog.data <data file>


[flit]: https://flit.readthedocs.io/en/latest/index.html
[fronttut]: ../tutorial/frontend-tut.md
