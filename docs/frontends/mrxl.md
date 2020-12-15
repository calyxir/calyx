# MrXL

MrXL is an example DSL for demonstrating FuTIL. MrXL programs consist of `map` and `reduce` operations on arrays. For example, this is a dot product implementation:

    input avec: int[1024]
    input bvec: int[1024]
    output dot: int
    prodvec := map 16 (a <- avec, b <- bvec) { a * b }
    dot := reduce 4 (a, b <- prodvec) 0 { a + b }

The numbers that come right after `map` and `reduce` are parallelism factors that guide the generation of hardware.


Install
-------

The MrXL implementation is in Python and uses [Flit][].
First, [install flit][flit] (`pip install flit` or similar), and then type the
following inside `frontend/mrxl`:

    flit install --symlink

This creates a symbolic link the mrxl directory and installs the `mrxl` command
line tool.

By default, [fud](../tools/fud.md) looks for the `mrxl` executable to enable
the `mrxl` compilation stage.
Type `fud check` to make sure `fud` reports that the `mrxl` compiler has been
found.


Interpreter
-----------

To run the interpreter, do this:

    mrxl <program> --data <indata> --interpret

where `<program>` is a MrXL source code file and `<indata>` is a JSON file containing values for all the variables declared as `input` in the program. The interpreter dumps the `output` variables as JSON to stdout.

You can try this, for example:

    mrxl test/dot.mrxl --data test/dot.json --interpret

Compiling to Calyx
------------------

To run the compiler, leave off the `--interpret` and `--data` flags:

    mrxl test/dot.mrxl

In order to run the compiler through `fud`, pass the `--from mrxl` flag:

    fud e --from mrxl <mrxl file> --to futil

To simulate the Verilog generated from the mrxl compiler, set the `-s
verilog.data` as usual:

    fud e --from mrxl <mrxl file> --to dat -s verilog.data <data file>


[flit]: https://flit.readthedocs.io/en/latest/index.html
