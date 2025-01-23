# Cider: The Calyx Interpreter & Debugger

Cider resides in the `interp/` directory of the
repository.
Cider supports all Calyx programs—from high-level programs that
make heavy use of control operators, to fully lowered Calyx programs.
(RTL simulation, in contrast, only supports execution of fully lowered programs.)

There are two ways to use the interpreter: you can directly invoke it, or you
can use [fud2][]. The latter is generally recommended.

## Basic Use

To run an example program, try:

    cd cider && cargo run -- tests/control/if.futil

You should see something like:

    ���T�itop_leveldmainhmemories��dnamecmemjdimensions�bD1fformat�fBitnum�fsigned�ewidth %


This output contains some header information and the raw binary data of the
memories in the program and as such is not human readable. A separate tool,
`cider-data-converter` is used to parse this dump into a human readable json and
vice versa. Once you've compiled it, either by running `cargo build` in
`tools/cider-data-converter` or by running `cargo build --all`, you can run:

    cargo run -- tests/control/if.futil | ../target/debug/cider-data-converter --to json

which should produce
```json
{
  "mem": [
    4
  ]
}
```



You can see the available command-line options by typing `cargo run -- --help`.

## Interpreting via fud

The interpreter is available as a stage in [fud2][], which lets you provide
standard JSON data files as input and easily execute passes on the input Calyx
program before interpretation.

You'll want to build the interpreter and compiler first:

    cargo build && \
    cd cider && cargo build && \
    cd ../tools/cider-data-converter && cargo build && cd ../../

or just run

    cargo build --all

Once you've installed and [configured](./fud2/index.md#configuration) `fud2` you
can run the same program by invoking

    fud2 tests/control/if.futil --to dat --through cider -s sim.data=tests/control/if.futil.data

Data is provided in the standard Calyx json and `fud2` will automatically handle
marshalling it to and from Cider's binary format, outputting the expected
result. Note that `fud2` _requires_ a provided data file, so in cases where you
do not initialize memory you will still need to provide the initial state of the
memories. Such files can be generated via the
[data gen tool](../tools/data-gen.md) or you can invoke Cider directly to bypass
this constraint.

By default, fud will not transform the Calyx code before feeding it to the interpreter.
To run passes before the interpreter, use the `calyx.flags` variable in conjunction with the `-p` flag.
For example, to fully lower the Calyx program before interpreting it:

    fud2 --to dat --through cider \
        -s calyx.flags='-p all' \
        -s sim.data=tests/control/if.futil.data \
        tests/control/if.futil

## Cider outputs
By default, Cider's output memory dump will only contain the `@external`
memories on the entrypoint component. If you want to see other memories in the
main component, the flag `--all-memories` will force Cider to serialize all
memories. For prototyping, it can occasionally be useful to serialize registers
as well, this can be done by passing the flag `--dump-registers` which will
cause Cider to serialize all registers in the main component as single entry
memories.



[fud2]: ./fud2/index.md
[ref-cells]: ../lang/memories-by-reference.md#the-easy-way-ref-cells
