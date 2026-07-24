
``auxin`` usage guide:

```
auxin [-f <from_fmt>] [-o <output-path>] [-t <to_fmt>] [-p <op>] [-x] [-e <dat-file-extension>] [--] [<input_path>]
```
if ``input_path`` is not given, ``auxin`` will attempt to read from stdin. if ``output_path`` is not given, ``auxin`` will output to stdout.


# format support

*if accepting input from stdin, the ``from_fmt`` flag **must** be set*

valid ``from_fmt``, ``to_fmt``:
- ``json``: the fud2 .data format
- ``cider, dump, data-dump``: the cider data dump format
- ``dat, verilog-dat, verilog, verilator, icarus``: the simulation hex directory format

``auxin`` can guess the the formats from ``input_path`` and ``output_path`` if the arguments are provided. these are mapped as follows:
- path is a directory: guesses hex directory
- extension is ``.dump``: guesses cider format
- extension is ``.data``: guesses json format

indicating formats with ``-f`` and ``-t`` will *always* take precedence over the guess.

# format specific controls

the ``-e`` flag can be used to set a file extension for entries in the data directory format. this flag controls which files ``auxin`` tries to read as input, and the names of files written out by ``auxin``. 

if the ``-x`` flag is set, json emitted will contain hexadecimal strings, i.e. ``"0x12345"``, rather than number literals. type information is retained.

# 'ops'

in the future, the ``-p`` option might be used to specify numeric transformations before data is written out. at the moment, it is effectively stubbed. do not use it.
