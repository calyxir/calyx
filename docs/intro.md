# Getting Started

FuTIL is an intermediate language and infrastructure for building compilers
that generate custom hardware accelerators.

FuTIL has many associated tools and depending on what you are doing, you
probably only need to install a subset of them.

We've organized this document into a core installation that is needed for everything
and then add-on installations for our command-line driver tool, simulation, frontends,
and synthesis.

## Core Installation

Start by cloning the repository:
```
git clone https://github.com/cucapra/futil.git
```

### Compiler dependencies
Install [Rust][rust] (it should automatically install `cargo`).

### Install tools for testing
  1. [runt][] hosts our testing infrastructure. Install with:
  `cargo install runt`
  2. [jq][]:
  Ubuntu: `sudo apt install jq`
  Mac: `brew install jq`
  Other platforms: [JQ installation][jq-install]

### Verify installation
Build the compiler with
```
cargo build
```
Then run the core tests with:
```
runt -i core
```

If everything has been installed correctly, this should not produce any failing
tests.

## Add-on Installations
Here are instructions for optional add-ons. We recommend installing at least `fud`.
### Fud (Command-line driver) [Recommended]
<details><summary>Click to expand!</summary>

[The FuTIL driver](./tools/fud.md) is a command line tool to drive the FuTIL
compiler and coordinate invoking frontends and backends.

You need [Flit][flit] to install `fud`.
```
pip3 install flit
```

Then install `fud` with:
```
cd fud
flit install -s
```

`fud` needs to know where the FuTIL directory lives. Running `fud check` will ask you
for the current directory and then display information about the tools that it could find.
```
fud check
```
</details>

### Simulation backend (Verilator)
<details><summary>Click to expand!</summary>

We use [Verilator][verilator] to simulate compiled designs and verify correctness. If you're on a Mac,
install with:
```
brew install verilator
```

Otherwise, you will probably need to compile it from source yourself (the versions in Linux repositories are generally out-of-date.)
There instructions are stolen from [Verilator install][verilator-install]:
```
git clone https://github.com/verilator/verilator
cd verilator
git pull
git checkout master
autoconf
./configure
make
sudo make install
```

Verilator can produce memory dumps and [VCD][] files (reporting the values of signals at every clock cycle).
Install `vcdump` so that `fud` can produce JSON representations of the VCD files for easier command-line
handling.
```
cargo install vcdump
```
</details>

### Python frontends (Systolic array, NTT, MrXL)
<details><summary>Click to expand!</summary>

You need [flit][] to install our python frontends.
```
pip3 install flit
```

Our python [frontends][frontends] use a FuTIL ast library written in python. Install with:
```
cd calyx-py && flit install -s
```

Frontend specific instructions:
 - [Systolic array](./frontends/systolic-array.md):
 Nothing else needed.
 - NTT: `pip3 install prettytables`
 - [MrXL](./frontends/mrxl.md): `cd frontends/mrxl && flit install -s`

</details>

### Dahlia frontend
<details><summary>Click to expand!</summary>

[Dahlia][dahlia] is an imperative HLS language that supports FuTIL as a backend.
[Here][dahlia-install] are the complete instructions, but we've provided a quick overview.
First, install [sbt][].
Then:
```
git clone https://github.com/cucapra/dahlia.git
cd dahlia
sbt assembly
```

If you have `fud` installed, tell `fud` where the Dahlia compiler lives:
```
fud config stages.dahlia.exec $(pwd)/fuse
```
</details>

### Vivado/VivadoHLS Synthesis backends
<details><summary>Click to expand!</summary>

We use Vivado to synthesis FuTIL designs and produce area and resource estimates.
There are two ways to get `fud` working with Vivado.

#### Vivado/VivadoHLS over SSH
`fud` supports invoking these tools over SSH. You have to tell `fud` the username and hostname
for a server that has these tools installed:
```
# vivado
fud config stages.synth-verilog.ssh_host <hostname>
fud config stages.synth-verilog.ssh_username <username>

# vivado hls
fud config stages.vivado-hls.ssh_host <hostname>
fud config stages.vivado-hls.ssh_username <username>
```

**Note:** `vivado` or `vivado_hls` have to be on the path of the remote machine for this
to work. If you need the names to be something else, file an issue. `fud` currently does
not support other names.

#### Vivado/VivadoHLS locally
We don't provide installation instructions for this. However, `fud` will look for
`vivado` and `vivado-hls` binaries on the system. If these are installed, you can
use `fud` to invoke these tools. You can change the paths `fud` looks for with
```
fud config stages.synth-verilog.exec <path> # update vivado path
fud config stages.vivado-hls.exec <path> # update vivado_hls path
```
</details>

## Where to go next?

### Try running an example program

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

### Check out more of our documentation

[How do I write a frontend for FuTIL?](./tutorial/frontend-tut.md)

[How do I write my own pass?](./compiler-docs.md)

[How does the language work?](./tutorial/language-tut.md)


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
[vcd]: https://en.wikipedia.org/wiki/Value_change_dump
[dahlia]: https://github.com/cucapra/dahlia
[dahlia-install]: https://github.com/cucapra/dahlia#set-it-up
[sbt]: https://www.scala-sbt.org/download.html
