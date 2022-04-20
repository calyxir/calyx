# Cider: Calyx Interpreter and Debugger


## Usage
For instructions and examples on using the interpreter functionality, see
[this page](https://docs.calyxir.org/interpreter.html) on the docs.

For examples of using the interactive debugging mode see [the docs](https://docs.calyxir.org/debug/cider.html).

## Compilation Options
By default cargo will compile cider with the `change-based-sim` feature enabled
which is a slightly more optimized version of the core simulation algorithm. You
can compile Cider with the naive simulation algorithm by disabling the default
feature:
```
cargo build --no-default-features
```

The release build of Cider is notably faster however since it uses rust's LTO
the compilation time takes a few minutes.
