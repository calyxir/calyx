## Setup

Run `make plugins` to build the builtin plugins (cocotb, verilator, etc.).

## Usage

There are two ways to use `tb`:

### Directly

For example, if you make sure to follow the instructions under [`examples/cocotb/doc_examples_quickstart/`](examples/cocotb/doc_examples_quickstart/),
```
make
./tb examples/cocotb/doc_examples_quickstart/my_design.sv -t examples/cocotb/doc_examples_quickstart/test_my_design.py --using cocotb
```
should run `cocotb` on the input file and harness.

### Via `fud2`:

You can follow the above steps but invoke the following command instead.
```
fud2 my_design.sv -s tb.test=test_my_design.py -s tb.using=cocotb --to tb
```

### Makefile

I've provided a [Makefile](Makefile) in this directory for local testing. Use `make` to build the `tb` executable locally.
