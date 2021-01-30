# Emitting Calyx from Python

Our frontends are written in python and make use of the `futil` library to
generate their code.

To install the library, run the following from the repository root (requires
[flit][] installation):
```
cd calyx-py && flit install -s
```

The library provides an example:
```
python calyx-py/test/example.py
```

[flit]: https://flit.readthedocs.io/en/latest/
