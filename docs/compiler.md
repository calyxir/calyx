# The Calyx Compiler

The Calyx compiler has several command line to control the execution of various
passes and backends.

### Controlling Passes

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
- collapse-control
- compile-control
...

Aliases:
- all: well-formed, papercut, remove-external-memories, ...
...
```

The first section list all the passes implemented in the compiler.
The second section lists *aliases* for combination of passes that are commonly
run together.
For example, the alias `all` is an ordered sequence of default passes executed
when the compiler is run from the command-line.

The command-line provides two options to control the execution of passes:
- `-p, --pass`: Execute this pass or alias. Overrides default alias.
- `-d, --disable-pass`: Disable this pass or alias. Takes priority over `-p`.

For example, we can run the following to disable the `static-timing` pass from
the default execution alias `all`:

```bash
cd futil
cargo run -- examples/futil/simple.futil -p all -d static-timing
```
