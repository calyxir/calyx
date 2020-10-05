# FUD: The FuTIL Driver

Working with FuTIL involves a lot of command-line tools. For example, an
incomplete yet daunting list of CLI tools used by FuTIL is:

- The Dahlia & the systolic array compilers
- FuTIL compiler and its various command line tools
- Verilator, the Verilog simulation framework used to test FuTIL-generated designs.
- Waveform viewers to see the results of simulation

`fud` aims to provide a simple interface for using these toolchains and
executing them in a pipeline.

The repository for `fud` is [here](https://github.com/cucapra/futil/tree/master/fud).

## Installation
You need [Flit](https://flit.readthedocs.io/en/latest/) to install `fud`. Install it with `pip3 install flit`.

You can then install `fud` with

```bash
flit install
```
(If using this method to install `fud`, `pip3` should be version >= 20)

You can also install `fud` with 

```bash
flit build
pip3 install dist/fud-0.1.0-py3-none-any.whl
```

If you are working on `fud` itself, you can install it with a symlink with:
```bash
flit install --symlink
```

### Installation of external tools
#### Dahlia
Dahlia is one of the frontends we support.
Compilation instructions are here: [Install Dahlia](https://github.com/cucapra/dahlia).
Once Dahlia is compiled, you need to configure `fud` so that it knows where to find
the binary.
In the Dahlia directory run:
```bash
fud config dahlia.stages.exec `pwd`/fuse
```


#### Verilator
We use the open source [Verilator](https://www.veripool.org/wiki/verilator) tool to simulate
FuTIL generated verilog. Installation instructions are here: [Install Verilator](https://www.veripool.org/projects/verilator/wiki/Installing)

#### Vcdump
Vcdump is a tool for converting `vcd` (Value Change Dump) files to json for easier analysis with the command line.
Install it with:
```bash
cargo install vcdump
```

## Usage
### Examples

```bash
# These commands will assume you're in the root directory for FuTIL.
$ cd futil

# Compile a Dahlia dot product implementation and simulate in verilog using the data provided.
# ========== Dahlia: examples/dahlia/dot-product.fuse
# ========== data:   examples/data/dot-product.data (`.data` is used as an extension alias for `.json`)
$ fud exec examples/dahlia/dot-product.fuse --to dat -s verilog.data examples/data/dot-product.data

# Compile and simulate a vectorized add implementation in FuTIL using the data provided,
# then dump the vcd into a new file for debugging.
# ========== FuTIL:   examples/futil/vectorized-add.futil
# ========== data:    examples/data/vectorized-add.data 
# ========== output:  v-add.vcd
$ fud exec examples/futil/vectorized-add.futil -o v-add.vcd -s verilog.data examples/data/vectorized-add.data

# Compile FuTIL source in the test vectorized-add.expect to Verilog.
# We must explicitly specify the input file type because it can not 
# be guessed from the extension.
$ fud exec examples/tests/vectorized-add.expect --from futil --to verilog

# Dry run of compiling the Dahlia dot product file to FuTIL. 
# As expected, this will *only* print the stages that will be run.
$ fud exec examples/dahlia/dot-product.fuse --to futil --dry-run
```

### Stages
`fud` transforms a file in one stage into a file in a later stage.
To do this, it needs to know the starting stage of the input file and the desired
destination stage.

`fud` will try to guess the starting stage by looking at the extension of the input file.
If it fails to guess correctly or doesn't know about the extension, you can manually set
the starting stage with the`--from` flag.

If the `-o` flag is passed to `fud`, then `fud` will use this extension to figure out the destination
stage. Similarly to the starting stage, you can always manually set this with the `--to` flag.
