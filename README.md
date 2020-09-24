# Fused Temporal Intermediate Language (FuTIL)

Intermediate language and compiler construction kit for hardware accelerators.

### Prerequisites

**Compiler dependencies**:
- Install [Rust][rust] (it should automatically install `cargo`).

**Testing dependencies**:
- [runt][] by running `cargo install runt`
- [vcdump][] by running `cargo install vcdump`
- [Verilator][]:
    - Ubuntu: `sudo apt install verilator`
    - Fedora: `sudo dnf install verilator`
    - Mac: `brew install verilator`
    - If none of these work for you, I refer you to the [official Verilator install instructions][verilator-install].
- [jq][]:
    - Ubuntu: `sudo apt install jq`
    - Fedora: `sudo dnf install jq`
    - Mac: `brew install jq`
    - If none of these work, here are [the official install directions][jq-install].

**Dahlia dependencies**:
- Install [Dahlia][].
- Point the `DAHLIA` environment variable to the Dahlia compiler binary

          export DAHLIA=<path-to-dahlia>/fuse

### Building and Testing

Build the compiler:

- Run `cargo build` to build the compiler.
- Run `./target/debug/futil --help` to get options from the `futil` binary.

Run the integration tests:

- Run `runt -x dahlia`

(Optional) Run the full test suite (requires installation of the Dahlia compiler):

- Run `runt`

### Using the Compiler

The compiler can be run in two ways:
1. `cargo run -- <options>`: Rebuild compiler and run the CLI.
2. `./target/debug/futil <options>`: Run the compiler without rebuilding the CLI.

Compile a FuTIL program to Verilog for Verilator simulation

          cargo run -- <file> -b verilog --verilator -l primitives/std.lib

### Compiler Development

The compiler source lives under [calyx/src](calyx/src). While changing the compiler,
use `cargo run -- <calyx options>` to rebuild the compiler and run files
at the same time.

**Primitives Library**: FuTIL can use customizable libraries based on the
backend. The libraries live under [primitives](primitives). You probably want
to use `primitives/std.lib` with the `-l` flag, which tells the compiler to look for ```primitives/std.lib```.
You'll need to explicitly pass the flag if you're not in the root directory. Assuming the import statement is ```import primitives/std.lib```:

```bash
$ cargo run -- examples/simple.futil -l primitives      # WRONG: "Failed to read primitives/primitives/std.lib"
$ cargo run -- examples/simples.futil                   # RIGHT

$ cd benchmarks                                         # Outside root directory 
$ cargo run -- ../examples/simple.futil -l ..           # RIGHT
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
[verilator-install]: https://www.veripool.org/projects/verilator/wiki/Installing
[jq]: https://stedolan.github.io/jq/
[jq-install]: https://stedolan.github.io/jq/
