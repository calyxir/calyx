# The Calyx Interpreter

The experimental Calyx interpreter resides in the `interp/` directory of the
repository.
The interpreter supports all Calyx programs—from high-level programs that
make heavy use of control operators, to fully lowered Calyx programs.
(RTL simulation, in contrast, only supports execution of fully lowered programs.)

There are two ways to use the interpreter: you can directly invoke it, or you can use [fud][].

## Basic Use

To run an example program, try:

    cd interp && cargo run tests/control/if.futil

You can see the available command-line options by typing `cargo run -- --help`.

## Interpreting via fud

The interpreter is available as a stage in [fud][], which lets you provide standard JSON data files as input and easily execute passes on the input Calyx program before interpretation.

You'll want to build the interpreter first:

    cd interp && cargo build

Here's how to run a Calyx program:

    fud e --to interpreter-out interp/tests/control/if.futil

To provide input data, set the `verilog.data` variable, like so:

    fud e --to interpreter-out \
        -s verilog.data tests/correctness/while.futil.data \
        tests/correctness/while.futil

By default, fud will not transform the Calyx code before feeding it to the interpreter.
To run passes before the interpreter, use the `calyx.flags` variable in conjunction with the `-p` flag.
For example, to fully lower the Calyx program before interpreting it:

    fud e --to interpreter-out \
        -s calyx.flags '-p all' \
        interp/tests/control/if.futil

[fud]: fud/index.md
