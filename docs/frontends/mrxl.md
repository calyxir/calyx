# MrXL

MrXL is an example DSL developed for the [frontend tutorial][fronttut].
MrXL programs consist of `map` and `reduce` operations on arrays.
For example, here is an implementation of dot-products:

    input avec: int[1024]
    input bvec: int[1024]
    output dot: int
    prodvec := map 16 (a <- avec, b <- bvec) { a * b }
    dot := reduce 4 (a, b <- prodvec) 0 { a + b }

The numbers that come right after `map` and `reduce` (`16` and `4` respectively) are "parallelism factors" that guide the generation of hardware.
The explanation on this page is relatively brief; see the [frontend tutorial][fronttut] for a more detailed explanation of the language. In particular, the [sum of squares][fronttut-sumsq] example is a good place to start.

Install
-------

You can run the MrXL implementation using [uv][].
Type this in the `mrxl` directory:

    uv run mrxl --help

[fud2][] also comes with an op for compiling MrXL programs to Calyx.

[uv]: https://docs.astral.sh/uv/
[fud2]: ../running-calyx/fud2

Interpreting MrXL
-----------------

To run the program through the MrXL interpreter, execute:

    mrxl <prog>.mrxl --data <prog>.mrxl.data --interpret

where `<prog>.mrxl` is a file containing MrXL source code and `<prog>.mrxl.data` is a file containing values for all the variables declared as `input`s in the MrXL program. The interpreter dumps the `output` variables, in JSON format, to stdout.

You could try, for example:

    mrxl test/dot.mrxl --data test/dot.mrxl.data --interpret

This is just a baby version of the dot-product implementation we showed at the very top; we have just shortened the input array so you can easily see it in full.
Similarly, we also provide `add.mrxl` and `sum.mrxl`, along with accompanying `<prog>.mrxl.data` files, under `test/`. Try playing with the inputs and the operations!


Compiling to Calyx
------------------

> The dot-product example above shows off features of MrXL that are not yet supported by the compiler. In particular, the compiler does not yet support `reduce` with a parallelism factor other than `1`. This is because MrXL is mostly a pedagogical device, and we want new users of Calyx to try implementing this feature themselves. To learn more about this and other extensions to MrXL, consider working through the [frontend tutorial][fronttut].

To run the compiler and see the Calyx code your MrXL program generates, just drop the `--data` and `--interpret` flags. For instance:

    mrxl test/dot.mrxl

In order to run the compiler through `fud`, pass the `--from mrxl` and `--to calyx` flags:

    fud2 --from mrxl <prog.mrxl> --to calyx

And finally, the real prize.
In order to compile MrXL to Calyx and then simulate the Calyx code in Verilog, run:

    mrxl --convert --data <prog>.mrxl.data > something.dat
    fud2 --from mrxl <prog>.mrxl --to dat --through verilator -s sim.data=something.dat

An aside: MrXL permits a simplified data format, which is what we have been looking at in our `<prog>.mrxl.data` files.
Files of this form need to be beefed up with additional information so that Verilog (and similar simulators) can work with them.

See this with:

    mrxl <prog>.mrxl --data <prog>.mrxl.data --convert

The output dumped to stdout is exactly this beefed-up data.
The changes it makes are:
1. It adds some boilerplate about the `format` of the data.
2. It infers the `output` variables required by the program and adds data fields for them.
3. It infers, for each memory, the parallelism factor requested by the program, and then divides the relevant data entries into _memory banks_.


[flit]: https://flit.readthedocs.io/en/latest/index.html
[fronttut]: ../tutorial/frontend-tut.html
[fronttut-sumsq]: ../tutorial/frontend-tut.html#example-sum-of-squares
