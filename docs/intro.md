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

- [runt][] by running `cargo install runt`
- [vcdump][] by running `cargo install vcdump`
- [Verilator][]:
    - Ubuntu: `sudo apt install verilator`
    - Fedora: `sudo dnf install verilator`
    - Mac: `brew install verilator`
    - Other platforms: [Verilator installation][verilator-install].
- [jq][]:
    - Ubuntu: `sudo apt install jq`
    - Fedora: `sudo dnf install jq`
    - Mac: `brew install jq`
    - Other platforms: [JQ installation][jq-install].

### Building and Testing

In the root of the repository, run the following:

- Run `cargo build` to build the compiler.
- Run `./target/debug/futil --help` to get options from the `futil` binary.
  Alternatively, run `cargo run -- --help` which rebuilds and runs the compiler.

Run the tests:
```
runt -x dahlia
```

### Running an Example Program

In order to run a FuTIL program, the run following from the repository:

```bash
cargo run -- examples/futil/simple.futil
```

This will run the FuTIL compiler on the input file `examples/futil/simple.futil`
and generate a FuTIL program without no control constructs.
In order to generate SystemVerilog, run the following:

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
