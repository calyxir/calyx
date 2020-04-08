# Fuse Temporal Intermediate Language (FuTIL)
An intermediate language for [Dahlia][].

**Note**: We are in the process of transitioning everything over to Rust. At the moment the interpreter is still in racket, although it hasn't been kept up to date with the syntax changes. `Calyx` is the name of the pass framework that we are writing. Instructions for installation are below.

### Installation
- Install [Rust][rust].

The core compiler can be built and installed with:
- Run `cargo build` to download all dependencies and build calyx.
- Run `./target/debug/calyx --help` to get options from the calyx binary.

We are using [runt][] for testing. Install it with:
- Install [runt][] by running `cargo install runt`.
- Run `runt` to run tests.

For Verilator testing, install:
- [verilator][]
  - Ubuntu: `sudo apt install verilator`
  - Fedora: `sudo dnf install verilator`
  - Mac: `brew install verilator`
  - If none of these work for you, I defer you to the official Verilator install
  instructions: [https://www.veripool.org/projects/verilator/wiki/Installing][]
- [vcdump][] by running `cargo install vcdump`
- [jq][]
  - Ubuntu: `sudo apt install jq`
  - Fedora: `sudo dnf install jq`
  - Mac: `brew install jq`
  - If none of these work, [here](https://stedolan.github.io/jq/download/) are the official install directions.

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

### Writing a Verilator test
The `tests/verilog` folder is dedicated to Verilator correctness testing.
A `.futil` in this directory defines a new test. To get output we run the following steps:
 - Use `verilog` backend to generate a Verilog file
 - Run Verilator using `sim/testbench.cpp` as the testbench. 
   - This expects a toplevel component in Futil named `main`
   - We drive the `main` component by pulsing the `clk` port on `main`
   - We simulate until the `ready` port on `main` is high.
 - Convert the generated `vcd` file to `json`
 - Use `{filename}.jq` to choose a subset of signals to test
 - Compare this generated file to `{filename.expect}`
 
Concretely, the things to do to create a new test are:
 - make a new `{filename}.futil` file
 - make a `{filename}.jq` file to select the signals you are interested in testing.
 **Note**: This file can be blank, but needs to exist.
 - make a `{filename}.expect` file with the values that you expect to see

## Makefile
`make [filename].futil`: Generate Futil program from Dahlia program. It
requires to install [Dahlia][] first.

`make [filename].v`: Generate Verilog RTL file from Futil program.

`make [filename].vcd`: Generate vcd file from Verilog RTL file. One can use ventilator to visualize it.

`make [filename].json`: Generate a json representation of the simulated Verilog file.

`make [filename].res`: Use `[filename].jq` to refine `[filename].json` to subset of signals.

`make clean`: Deletes all generated files.

[rust]: https://doc.rust-lang.org/cargo/getting-started/installation.html
[dahlia]: https://github.com/cucapra/dahlia
[structopt]: https://docs.rs/structopt/0.3.11/structopt/
[runt]: https://github.com/rachitnigam/runt
[vcdump]: https://github.com/sgpthomas/vcdump
[verilator]: https://www.veripool.org/wiki/verilator
