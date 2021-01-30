# Getting Started

FuTIL is an intermediate language and infrastructure for building compilers
that generate custom hardware accelerators.

The following instructions will install all dependencies required to build
the compiler and run the compiler tests.

Start the process by cloning the repository:
```
git clone https://github.com/cucapra/futil.git
```

### Compiler dependencies
Install [Rust][rust] (it should automatically install `cargo`).

### Testing dependencies
The compiler uses expectation tests as well as hardware simulation tests.

1. [runt][] by running `cargo install runt`
2. [vcdump][] by running `cargo install vcdump`
3. [Verilator][]:
    - Ubuntu: `sudo apt install verilator`
    - Fedora: `sudo dnf install verilator`
    - Mac: `brew install verilator`
    - Other platforms: [Verilator installation][verilator-install].
4. [jq][]:
    - Ubuntu: `sudo apt install jq`
    - Fedora: `sudo dnf install jq`
    - Mac: `brew install jq`
    - Other platforms: [JQ installation][jq-install].
5. (Optional) [flit][]:
    - `python -m pip install flit`

### Install the FuTIL Driver

[The FuTIL driver](./tools/fud.md) is required to run the various tests.
Follow the [installation instructions](./tools/fud.html#installation):
```
cd fud && flit install -s
```

### Install the FuTIL Python Library

[Frontend compilers][frontends] use a [Python library][calyx-py] to emit FuTIL
programs:
```
cd calyx-py && flit install -s
```

### Building and Testing

In the root of the repository, run the following:

1. Run `cargo build` to build the compiler.
2. Run `./target/debug/futil --help` to get options from the `futil` binary.
  Alternatively, run `cargo run -- --help` which rebuilds and runs the compiler.

3. Run the tests (excluding the test for the frontend compilers):
```bash
runt --exclude frontend
```

### Running an Example Program

In order to run a FuTIL program, execute the following command from the repository:

```bash
cargo run -- examples/futil/simple.futil
```

This will run the FuTIL compiler with input file `examples/futil/simple.futil`,
and generate a FuTIL program without control constructs.
In order to generate SystemVerilog, execute the following:

```bash
cargo run -- examples/futil/simple.futil -b verilog
```

[rust]: https://doc.rust-lang.org/cargo/getting-started/installation.html
[runt]: https://github.com/rachitnigam/runt
[vcdump]: https://github.com/sgpthomas/vcdump
[verilator]: https://www.veripool.org/wiki/verilator
[verilator-install]: https://www.veripool.org/projects/verilator/wiki/Installing
[jq]: https://stedolan.github.io/jq/
[jq-install]: https://stedolan.github.io/jq/
[frontends]: ./frontends/index.md
[calyx-py]: ./calyx-py.md
[flit]: https://flit.readthedocs.io/en/latest/
