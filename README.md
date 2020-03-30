# Fuse Temporal Intermediate Language (FuTIL)
An intermediate language for [Dahlia][].

**Note**: We are in the process of transitioning everything over to Rust. At the moment the interpreter is still in racket, although it hasn't been kept up to date with the syntax changes. `Calyx` is the name of the pass framework that we are writing. Instructions for installation are below.

### Installation
- Install [Rust][rust].
- Install [runt][] by running `cargo install runt`.
- Run `cargo build` to download all dependencies and build calyx.
- Run `runt` to run tests.
- Run `./target/debug/calyx --help` to get options from the calyx binary.

### Compiler Development

The compiler source lives under [src](src). While changing the compiler,
use `cargo run -- <calyx options>` to rebuild the compiler and run files
at the same time.

**Primitives Library**: FuTIL can use customizable libraries based on the
backend. The libraries live under [primitives](primitives). You probably want
to use `primitives/std/lib` with the `-l` flag.

```bash
cargo run -- examples/simple.futil -l primitives/std.lib
```

**Debug mode**: The `-d` flag shows the FuTIL program after running a pass.
Use this to debug outputs from passes.

The general flow of the compiler is as follows:
 1) Build up `context::Context`
    - Parse FuTIL files + library file specified in CLI.
    - Build an in memory representation of each Component specified using
      the primitives defined in the library file to resolve and instantiate
      subcomponents defined with `new-std` statements.
 2) Run passes that update the context. Passes are defined using the Visitor
    framework defined in `src/passes/visitor.rs`. For now the passes to run are
    just specified in `src/main.rs`
 3) Use a backend to emit code from the context. Backends must implement
    `backend::traits::Backend`.

We use the [structopt][] library to
implement the command line interface.

## Makefile

`make [filename].futil`: Generate Futil program from Dahlia program. It
requires to install [Dahlia][] first.

`make [filename].v`: Generate Verilog RTL file from Futil program.

`make [filename].vcd`: Generate vcd file from Verilog RTL file. One can use ventilator to visualize it.

`make clean`: Deletes all generated files.

[rust]: https://doc.rust-lang.org/cargo/getting-started/installation.html
[dahlia]: https://github.com/cucapra/dahlia
[structopt]: https://docs.rs/structopt/0.3.11/structopt/
[runt]: https://github.com/rachitnigam/runt
