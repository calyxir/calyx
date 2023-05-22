# The Calyx Compiler

The Calyx compiler has several command line options to control the execution of
various passes and backends.

## Controlling Passes

The compiler is organized as a sequence of passes that are run when the compiler
executes.
To get a complete list of all passes, run the following from the repository
root:

```
cargo run -- --list-passes
```

This generates results of the form:

```
Passes:
- collapse-control: <description>
- compile-control: <description>
...

Aliases:
- all: well-formed, papercut, remove-external-memories, ...
...
```

The first section lists all the passes implemented in the compiler.
The second section lists *aliases* for combinations of passes that are commonly
run together.
For example, the alias `all` is an ordered sequence of default passes executed
when the compiler is run from the command-line.

The command-line provides two options to control the execution of passes:
- `-p, --pass`: Execute this pass or alias. Overrides default alias.
- `-d, --disable-pass`: Disable this pass or alias. Takes priority over `-p`.

For example, we can run the following to disable the `static-timing` pass from
the default execution alias `all`:

```bash
cargo run -- examples/futil/simple.futil -p all -d static-timing
```

## Providing Pass Options

Some passes take options to control their behavior. The `--list-passes` command prints out the options for each pass. For example, the `tdcc` pass has the following options:

```
tdcc: <description>
  * dump-fsm: Print out the state machine implementing the schedule
```

The option allows us to change the behavior of the pass. To provide a pass-specific option, we use the `-x` switch:
```
cargo run -- examples/futil/simple.futil -p tdcc -x tdcc:dump-fsm
```

Note that we specify the option of `tdcc` by prefixing it with the pass name and a colon.


## Specifying Primitives Library

The compiler implementation uses a standard library of components to compile
programs.
The only standard library for the compiler is located in:
```
<path to Calyx repository>/primitives
```

Specify the location of the library using the `-l` flag:
```
cargo run -- -l ./primitives
```

## Primitive Libraries Format

The primitive libraries consist of a `.futil` file paired with a `.sv` file. The
`.futil` file defines a series of Calyx shim bindings in `extern` blocks which
match up with SystemVerilog definitions of those primitives. These libraries may
also expose components written in Calyx, usually defined using primitives
exposed by the file.

No Calyx program can work without the primitives defined in the [Core Library](libraries/core.md).