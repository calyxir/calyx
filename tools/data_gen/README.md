# data generator

a program which uses ``num_ir`` to generate ``n`` random numbers of a given type.

## usage

```
  cargo run -p data_gen <TYPE> <NUM_ITEMS>
```

- required args
  - type: consult ``fixed_cli``'s docs. this program uses the same type of shorthand type notation.
  - num_items: the number of entries to generate.
- optional args
  - ``-o``: out_path: a path to output results to.
  - ``--fixed-bound <decimal>``: if the type specified is fixed-point, restrict generated values to within this decimal value. if the type is signed, the bound will be ``-bound..bound``. if the type is unsigned, the bound will be ``0..bound``. decimal does not have to be quoted.
  - ``-d``: RNG seed. expects a u64.
  - ``-s, --sep <string>``: separator between elements. defaults to ``\n``. setting a new separator will not include a newline by default.
  - ``-x``: print generated entries as hex strings, rather than pretty printing
  - ``-e``: if ``-x`` is active, also pretty-print entries to stderr.
