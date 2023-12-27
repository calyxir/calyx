# Systolic Array

Systolic arrays are commonly used to implement fast linear-algebra
computations. See [this paper][kung-systolic] for an overview on
systolic arrays.

The systolic array frontend lives in the [systolic-lang][] folder in the
Calyx repository and generates systolic arrays that can perform matrix
multiplies.

The `gen-systolic.py` contains the entire program required to generate
systolic arrays. In order to generate an *8 X 8* systolic array, run:

```
./frontends/systolic-lang/gen-systolic.py -tl 8 -td 8 -ll 8 -ld 8
```

## Installation

Install the [calyx-py](../calyx-py.md) library.

## Command Line Options

The command line options configure the dimensions of the generated
systolic array. There are no other properties of the systolic array that
can be configured.

- `--top-length`, `--left-length`: The length of top and left sides of the systolic array.
- `--top-depth`, `--left-depth`: The length of the input streams from top and left sides of the array.
- `--post-op`: Specify the post operation to be performed on the result of the matrix-multiply.
- `--fixed-dim`: Generate systolic array that only processes matrices with the given sizes. The default strategy supports matrices with any contraction dimension.

[kung-systolic]: http://www.eecs.harvard.edu/~htk/publication/1982-kung-why-systolic-architecture.pdf
[systolic-lang]: https://github.com/calyxir/calyx/tree/master/frontends/systolic-lang
