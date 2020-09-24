# FUD: FuTIL Driver
This is the FuTIL driver. It is a tool that automates the process
of calling FuTIL frontends, the FuTIL compiler, and any backends that may
be needed to simulate/execute a program.

## Installation
You can install `fud` with `./setup.py install --user`.

### Installation of external tools
#### Dahlia
#### Verilator
#### Vcdump

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

#
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
