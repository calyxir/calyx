# FUD: FuTIL Driver
This is the FuTIL driver. It is a tool that automates the process
of calling FuTIL frontends, the FuTIL compiler, and any backends that may
be needed to simulate/execute a program.

## Installation
You need [Flit](https://flit.readthedocs.io/en/latest/) to install `fud`.
Install it with `pip3 install flit`.

Once that's installed, install `fud` with:
```bash
flit install
```

If you are working on `fud` itself, you can install it with a symlink with:
```bash
flit install --symlink
```

### Installation of external tools
#### Dahlia
Dahlia is one of the frontends we support. Installation instructions are here: [Install Dahlia](https://github.com/cucapra/dahlia)

#### Verilator
We use the open source [Verilator](https://www.veripool.org/wiki/verilator) tool to simulate
FuTIL generated verilog. Installation instructions are here: [Install Verilator](https://www.veripool.org/projects/verilator/wiki/Installing)

#### Vcdump
Vcdump is a tool for converting `vcd` (Value Change Dump) files to json for easier

## Usage
### Quickstart

```bash
# compile and simulate a Dahlia dot-produce implementation using the data in data/dot-product.json
fud exec dot-product.fuse --to dat -s verilog.data data/dot-product.json

# compile and simulate a matrix add implementation in Futil and dump the vcd for debugging
# using data in data/mat-add.json for simulation
fud exec mat-add.futil -o mat-add.vcd -s verilog.data data/mat-add.json

# compile Futil source in par.expect to Verilog
# we explicilty specify the input file type because
# it can not be guessed from the extension
fud exec tests/par.expect --from futil --to verilog

# dry run of simulatining a Dahlia dot-product implementation. This will print
# the commands that will be run, but not do anything.
fud exec dot-product.fuse --to dat -s verilog.data data/dot-product.json --dry-run
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
