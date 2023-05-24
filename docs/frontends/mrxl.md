# MrXL

MrXL is an example DSL developed for the [frontend tutorial][fronttut].
MrXL programs consist of `map` and `reduce` operations on arrays.
For example, here is a dot-product implementation:

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

    mrxl <prog>.mrxl --data <prog>.mrxl.data --interpret

where `<prog>.mrxl` is a file containing MrXL source code and `<prog>.mrxl.data` is a file containing values for all the variables declared as `input`s in the MrXL program. The interpreter dumps the `output` variables, in JSON, to stdout.

You could try, for example:

    mrxl test/dot.mrxl --data test/dot.mrxl.data --interpret

This is just a baby version of the dot-product implementation we showed at the very top; we have just shortened the input array so you can easily see it in full.
We also provide `add.mrxl` and `sum.mrxl`, along with sample `<prog>.mrxl.data` files, under `test/`. Try playing with the inputs and the operations!


Compiling to Calyx
------------------

To run the compiler and see the Calyx code your MrXL program generates, just drop the `--data` and `--interpret` flags. For instance:

    mrxl test/dot.mrxl

In order to run the compiler through `fud`, pass the `--from mrxl` and `--to futil` flags:

    fud e --from mrxl <prog.mrxl> --to calyx

And finally, the real prize.
In order to compile MrXL to Calyx and then simulate the Calyx code in Verilog, run:

    fud e --from mrxl <prog>.mrxl --to dat --through verilog -s mrxl.data <prog>.mrxl.data

An aside: MrXL permits a simplified data format, which is what we have been looking at in our `<prog>.mrxl.data` files.
Files of this form need to be beefed up with additional information so that Verilog (and similar simulators) can work with them.
We did this beefing up "on the fly" in the incantation above, but it is interesting to see the changes we made.

See this with:

    mrxl <prog>.mrxl --data <prog>.mrxl.data --convert

The output dumped to stdout is exactly this beefed-up data.
The changes it makes are:
1. It adds some boilerplate about the `format` of the data.
2. It infers the `output` variables required by the program and adds data fields for them.
3. It infers, for each memory, the parallelism factor requested by the program, and then divides the relevant data entries into _memory banks_.


[flit]: https://flit.readthedocs.io/en/latest/index.html
[fronttut]: ../tutorial/frontend-tut.md

