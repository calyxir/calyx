# `exp` Generator

The `exp` generator uses a Taylor series approximation to calculate the fixed point value of the natural
exponential function `e^x`. The [Maclaurin series](https://en.wikipedia.org/wiki/Taylor_series#Exponential_function) 
for the function can be written as:
```
e^x = 1 + x + x^2/2! + x^3/3! + ... + x^n/n!
```
where `n` is the nth degree or order of the polynomial.

For signed values, we can take the reciprocal value:
```
e^(-x) = 1/e^x
```

The `gen_exp.py` file can generate an entire Calyx program for testing purposes.
The `main` component contains memories `x` (for the input) and `ret` for the result of `e^x`. 
In order to generate an example program with degree `4`, bit width `32`, integer bit width `16`, and `x` interpreted as a signed value:

```
./calyx-py/calyx/gen_exp.py -d 4 -w 32 -i 16 -s true
```

Similarly, it provides a function to produce only the necessary components to be dropped into other Calyx programs.

## Installation

Install the [calyx-py](../calyx-py.md) library.

## Command Line Options

The command line options configure the degree (or order) of the taylor series, bit width, integer bit width, and sign.

- `--degree`: The degree of the Taylor polynomial.
- `--width`: The bit width of the value `x`.
- `--int_width`: The integer bit width of the value `x`. The fractional bit width is then inferred as `width - int_width`.
- `--is_signed`: The signed interpretation of the value `x`. If `x` is a signed value, this should be `true` and otherwise, `false`. 
