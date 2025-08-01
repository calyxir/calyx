# Getting Started

Calyx is an intermediate language and infrastructure for building compilers
that generate custom hardware accelerators.
These instructions will help you set up the Calyx compiler and associated
tools.
By the end, you should be able to compile and simulate hardware designs
generated by Calyx.

## Compiler Installation

There are three possible ways to install Calyx, depending on your goals.

### Using Docker

The easiest way is to use the [Calyx Docker image][calyx-docker] that provides a pinned version of the compiler, all frontends, as well as configuration for several tools.

The following commands will fetch the Docker image and start a container with an interactive shell:

```sh
docker run -it --rm ghcr.io/calyxir/calyx:latest
```

The `--rm` flag will remove the container after you exit the shell. If you want to keep the container around, remove the flag.

You can skip forward to [*running a hardware design*][hw-design].

### Installing the Crate (to use, but not extend, Calyx)

First, install [Rust][rust].
This should automatically install `cargo`.

If you want just to play with the compiler, install the [`calyx` crate][calyx-crate]:

```
cargo install calyx
```

This will install the `calyx` binary which can optimize and compile Calyx programs. You will still need the [`primitives/core.futil`][core-lib] and [its accompanying Verilog file](https://github.com/calyxir/calyx/blob/master/primitives/core.sv) library to compile most programs.

### Installing from Source (to use and extend Calyx)

First, install [Rust][rust].
This should automatically install `cargo`.

Clone the repository:

```
git clone https://github.com/calyxir/calyx.git
```

Then build the compiler:

```
cargo build
```

You can build and run the compiler with:

```
cargo build # Builds the compiler
./target/debug/calyx --help # Executes the compiler binary
```

We recommend installing the git hooks to run linting and formatting checks before each commit:

```shell
/bin/sh setup_hooks.sh
```

You can build the docs by installing `mdbook` and the callouts preprocessor:

```sh
/bin/sh docs/install_tools.sh
```

Then, run `mdbook serve` from the project root.

## Running Core Tests

The core test suite tests the Calyx compiler's various passes.
Install the following tools:

  1. [runt][] hosts our testing infrastructure. Install with:
  `cargo install runt`
  2. [jq][] is a command-line JSON processor. Install with:
     * Ubuntu: `sudo apt install jq`
     * Mac: `brew install jq`
     * Other platforms: [JQ installation][jq-install]

Build the compiler:

```
cargo build
```

Then run the core tests with:

```
runt -i core
```

If everything has been installed correctly, this should not produce any failing
tests.

## Installing the Command-Line Driver

[The Calyx driver](./running-calyx/fud2), called fud2, wraps the various compiler frontends and
backends to simplify running Calyx programs.
Start at the root of the repository.

Install fud2 using Cargo:

    cargo install --path fud2

fud2 requires [Ninja][] and [uv][], so install those if you don't already have them.
For example, use `brew install ninja uv` on macOS or `apt-get install ninja-build` followed by `curl -LsSf https://astral.sh/uv/install.sh | sh` on Debian/Ubuntu.

Configure fud2 by typing:

    fud2 edit-config

And put this in the resulting TOML file:

```toml
[calyx]
base = "<path to Calyx repository>"
```

Finally, use this to set up fud2's Python environment:

    fud2 env init

You can read [more about setting up and using fud2][fud2] if you're curious.

[ninja]: https://ninja-build.org/manual.html
[uv]: https://docs.astral.sh/uv/

## Simulation

There are three ways to run Calyx programs:
[Verilator][], [Icarus Verilog][], and Calyx's native [interpreter][].
You'll want to set up at least one of these options so you can try out your code.

Icarus Verilog is an easy way to get started on most platforms.
On a Mac, you can install it via [Homebrew][] by typing `brew install icarus-verilog`.
On Ubuntu, [install from source][icarus-install-source].

It is worth saying a little about the alternatives.
You could consider:

1. [Setting up Verilator][fud-verilator] for faster performance, which is good for long-running simulations.
2. Using the [interpreter][] to avoid RTL simulation altogether.

## Running a Hardware Design

You're all set to run a Calyx hardware design now. Run the following command:

```
fud2 examples/tutorial/language-tutorial-iterate.futil \
  -s sim.data=examples/tutorial/data.json \
  --to dat --through icarus
```

(Change the last bit to `--through verilator` to use Verilator instead.)

This command will compile `examples/tutorial/language-tutorial-iterate.futil` to Verilog
using the Calyx compiler, simulate the design using the data in `examples/tutorial/data.json`, and generate a JSON representation of the
final memory state.

Congratulations! You've simulated your first hardware design with Calyx.

## Where to go next?

* [How can I setup syntax highlighting in my editor?](./tools/editor-highlighting.md)
* [How does the language work?](./tutorial/language-tut.md)
* [Where can I see further examples with `fud2`?](./running-calyx/fud2#general-use)
* [How do I write a frontend for Calyx?](./tutorial/frontend-tut.md)

[rust]: https://doc.rust-lang.org/cargo/getting-started/installation.html
[runt]: https://github.com/rachitnigam/runt
[verilator]: https://www.veripool.org/wiki/verilator
[icarus verilog]: http://iverilog.icarus.com
[jq]: https://stedolan.github.io/jq/
[jq-install]: https://stedolan.github.io/jq/
[interpreter]: ./running-calyx/interpreter.md
[homebrew]: https://brew.sh
[fud-icarus]: ./running-calyx/fud/index.md#icarus-verilog
[fud-verilator]: ./running-calyx/fud/index.md#verilator
[icarus-install-source]: https://iverilog.fandom.com/wiki/Installation_Guide#Installation_From_Source
[calyx-crate]: https://crates.io/crates/calyx
[core-lib]: https://github.com/calyxir/calyx/blob/master/primitives/core.futil
[calyx-docker]: https://github.com/calyxir/calyx/pkgs/container/calyx
[hw-design]: ./intro.md#running-a-hardware-design
