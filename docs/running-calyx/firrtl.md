# FIRRTL Backend

Calyx programs can be translated into the [FIRRTL](https://bar.eecs.berkeley.edu/projects/firrtl.html) intermediate language. <!-- TODO: Link OSDA paper when it goes on arxiv -->

## Basic Use

To translate an example program to FIRRTL, try:

    $ cargo run examples/tutorial/language-tutorial-iterate.futil -p external-to-ref -p all -b firrtl

## Running with `fud2`

The FIRRTL backend can also be run through fud2, which we recommend using.

### Setup

To run FIRRTL-translated programs, we need to set up [Firtool](https://github.com/llvm/circt) for use by fud2. We recommend using [Firtool version 1.75.0](https://github.com/llvm/circt/releases/tag/firtool-1.75.0).

First, download and extract the Firtool binary. Then, edit `fud2`'s configuration file:

    $ fud2 edit-config

Add these lines:

```toml
[firrtl]
firtool = "<path to extracted firtool directory>/bin/firtool"
```

[fud2]: ./fud2.md

### Obtaining FIRRTL

> The FIRRTL backend on fud2 currently requires Calyx with the YXI feature to be built. (Refer to the above)

The FIRRTL backend offers two options based on how Calyx primitives are handled: (1) use Calyx's existing Verilog implementations, and (2) generate FIRRTL implementations.

To generate FIRRTL-version of the Calyx program that will use Verilog primitives, run fud2 with `--to firrtl`:
```
fud2 examples/tutorial/language-tutorial-iterate.futil --to firrtl
```

To generate FIRRTL-version of the Calyx program containing FIRRTL primitives, run fud2 with `--to firrtl-with-primitives`:
```
fud2 examples/tutorial/language-tutorial-iterate.futil --to firrtl-with-primitives
```

### Simulating FIRRTL-translated programs

To simulate a FIRRTL-translated Calyx program using Verilog primitives, run fud2 with `--through firrtl`:
```
fud2 examples/tutorial/language-tutorial-iterate.futil --to dat -s sim.data=examples/tutorial/data.json --through firrtl
```

To simulate a FIRRTL-translated Calyx program using FIRRTL primitives, run fud2 with `--through firrtl-with-primitives`:

```
fud2 examples/tutorial/language-tutorial-iterate.futil --to dat -s sim.data=examples/tutorial/data.json --through firrtl-with-primitives
```

Both examples will yield
```
{
  "cycles": 76,
  "memories": {
    "mem": [
      42
    ]
  }
}
```

### Adding SystemVerilog Primitives for `firrtl`

To add primitives to `firrtl`, you would add the SystemVerilog implementation of the primitive to the file, `fud2/rsrc/primitives-for-firrtl.sv`

### Adding Firrtl Primitives for `firrtl-with-primitives`

1.  **Create a Template:** Add a `.fir` file to `tools/firrtl/templates/`. The filename should match the primitive name (e.g., `std_add.fir`).
2.  **Define the Interface:** The template must define a `module` whose ports (names and widths) match the [Calyx primitive library](https://docs.calyxir.org/libraries/core.html).
3.  **Update the Replacement Map:** If your primitive uses unique parameters, update the replacement map logic in `tools/firrtl/generate-firrtl-with-primitives.py`. 
4.  **Testing:** Add a `runt` test in `tests/firrtl/primitive-templates` to ensure the new implementation generates valid FIRRTL and produces correct simulation results.
