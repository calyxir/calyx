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

## preparing data for simulation / using the data generator with ``auxin``

with the ``-x`` flag, an output file can be used as a ``.dat`` without modification. make sure that the output file has the name of the memory you wish to provide data for.

to obtain a ``.data`` file:
- one option is to use ``-s`` to comma-separate the values, and then paste the contents into the data field.
- another is to manually write a ``header`` file, generate a ``.dat``-style output, and then use ``auxin`` to perform the conversion
  - the header format is comma-separated like: ``memory_name,type,length``. no internal spaces.
