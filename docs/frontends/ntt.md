# Number Theoretic Transform (NTT)

The number theoretic transform is a generalization of the
fast Fourier transform that uses nth primitive root of unity
based upon a quotient ring instead of a field of complex numbers.

It is commonly used to speed up computer arithmetic, such as the
multiplication of large integers and large degree polynomials. The
pipeline produced here is based upon [this paper][longa-etal-ntt],
which also provides some background information on NTT.

The NTT pipeline frontend lives in the [ntt][] folder in the
Calyx repository and generates the pipeline for the NTT transform.

The `gen-ntt-pipeline.py` file contains the entire program required to
generate NTT pipelines. In order to generate a pipeline with
bit width `32`, input size `4`, and modulus value `97`:

```
./frontends/ntt-pipeline/gen-ntt-pipeline.py -b=32 -n=4 -q=97
```

## Installation

Install the [calyx-py](../calyx-py.md) library.

The generator also produces a table to illustrate which operations are occurring
during each stage of the pipeline. This requires installing PrettyTable:

    pip3 install prettytable numpy

## Fud Stage

The NTT pipeline defines an [external fud stage][../running-calyx/fud/external.md] to
transform configuration files into Calyx programs.
To install, run:

```
fud register ntt -p frontends/ntt-pipeline/fud/ntt.py && fud check
```

This should report the newly installed `ntt` stage in the configuration.

## Configuration Files

Configurations files simply specify command line parameters:
```json
{
  "input_bitwidth": 32,
  "input_size": 4,
  "modulus": 97
}
```

## Command Line Options

The command line options configure the bit width, size, and modulus value of the
pipeline.

- `--input_bitwidth`: The bit width of each value in the input array.
- `--input_size`: The length (or size) of the input array.
- `--modulus`: The (prime) modulus value used during the transformation.
- `--parallel_reduction`: Decreases fan-out by reducing the number of groups executed in parallel by this factor.

[longa-etal-ntt]: https://www.microsoft.com/en-us/research/wp-content/uploads/2016/05/RLWE-1.pdf
[ntt]: https://github.com/calyxir/calyx/tree/master/frontends/ntt-pipeline
