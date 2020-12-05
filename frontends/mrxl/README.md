MrXL
====

MrXL is an example DSL for demonstrating FuTIL. MrXL programs consist of `map` and `reduce` operations on arrays. For example, this is a dot product implementation:

    input avec: int[1024]
    input bvec: int[1024]
    output dot: int
    prodvec := map 16 (a <- avec, b <- bvec) { a * b }
    dot := reduce 4 (a, b <- prodvec) 0 { a + b }

The numbers that come right after `map` and `reduce` are parallelism factors that guide the generation of hardware.


Install
-------

The MrXL implementation is in Python and uses [Poetry][]. First, [install Poetry](https://python-poetry.org/docs/#installation) (`pip install poetry` or similar), and then type:

    poetry install

You'll also want to do this so you can use `mrxl` as a stage of `fud`:

    poetry build
    pip install dist/<generated_wheel_file>

[poetry]: https://python-poetry.org


Interpreter
-----------

To run the interpreter, do this:

    poetry run mrxl <program> <indata>

where `<program>` is a MrXL source code file and `<indata>` is a JSON file containing values for all the variables declared as `input` in the program. The interpreter dumps the `output` variables as JSON to stdout.

You can try this, for example:

    poetry run mrxl test/dot.mrxl test/dot.json


Tests
-----

There are tests using [Runt][]. Just install Runt (`cargo install runt`) and type `runt` to run them.

[runt]: https://github.com/rachitnigam/runt
