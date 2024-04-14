# Data Format

Calyx's [`fud`][fud]-based workflows specifies a JSON-based data format which can be used with software simulators.

## External memories

First, a Calyx program must mark memories using the [`@external` attribute][ext-attr] to tell the compiler that the memory is used as either an input or an ouput.

```
component main() {
    cells {
        @external ext = comb_mem_d1(...);
        internal = comb_mem_d1(...);
    }
}
```

In the above program, the memory `ext` will be assumed to be an input-output memory by the compiler while `internal` is considered to be an internal memory.
For all external memories, the compiler will generate code to read initial values and dump out final values.
The external attribute is recognized for all the `std_mem` and `seq_mem` primitives.
The `@external` attribute can **only be used** on the [top-level component][toplevel-attr].

> When the `--synthesis` flag is passed to the compiler, this behavior is disabled and instead the memories are turned into ports on the top-level component.


## The Data Format

The JSON-based data format allows us to provide inputs for a memory and specify how the values should be interpreted:
```json
{{#include ../../examples/tutorial/data.json}}
```

The `data` field represents the initial data for the memory and can use mutlidimensional arrays to describe it. For example, the following is the initial data for a two-dimensional (`comb_mem_d2`) memory:

```json
{
    "data": [ [1, 2], [3, 4] ],
    "format": {...}
}
```

The `format` specifier tells `fud` how to interpret the values in the data field. The original program specifies that all values should be treated as 32-bit, unsigned values.

In order to specify fixed-point values, we must specify both the total width and fraction widths:
```json
"root": {
  "data": [
      0.0
  ],
  "format": {
      "numeric_type": "fixed_point",
      "is_signed": false,
      "width": 32,
      "frac_width": 16
  }
}
```
The format states that all values have a fractional width of 16-bits while the remainder is used for the integral part.

> **Note:** `fud` requires that for each memory marked with the `@external` attribute in the program.

## Using `fud`

All software simulators supported by `fud`, including [Verilator][] and [Icarus Verilog][iv], as well as the [Calyx interpreter][interpreter] can use this data format.
To pass a JSON file with initial values, use the `-s verilog.data` flag:

```bash
# Use Icarus Verilog
fud e --to dat --through icarus-verilog <CALYX FILE> -s verilog.data <JSON>
# Use Verilator
fud e --to dat --through verilog <CALYX FILE> -s verilog.data <JSON>
# Use the Calyx Interpreter
fud e --to interpreter-out <CALYX FILE> -s verilog.data <JSON>
```

## Generating Random Values

Often times, it can be useful to automatically generate random values for a large memory. The [data-gen][] tool takes a Calyx program as an input and automatically generates random values for each memory marked with `@external` in the above data format.



[toplevel-attr]: attributes.md#toplevel
[ext-attr]: attributes.md#external
[fud]: ../running-calyx/fud/index.md
[data-gen]: ../tools/data-gen.md
[iv]: ../running-calyx/fud/index.md#icarus-verilog
[verilator]: ../running-calyx/fud/index.md#verilator
[interpreter]: ../interpreter.md