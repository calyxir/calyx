# exp Generator

The `exp` generator uses a Taylor series approximation to calculate the value of the natural
exponential function `e^x`. The [Maclaurin series](https://en.wikipedia.org/wiki/Taylor_series#Exponential_function) 
for the function can be written as:
```
e^x = 1 + x + x^2/2! + x^3/3! + ... + x^n/n!
```
where `n` is the nth degree or order of the polynomial.

The `gen_exp.py` file can generate an entire Calyx program for testing purposes.
The `main` component contains memories `x` (for the input) and `ret` for the result of `e^x`. 
In order to generate an example program with degree `4`, bit width `32`, and `integer bit width` 16:

```
./calyx-py/calyx/gen_exp.py -d 2 -w 32 -i 16
```

Similarly, it provides a function to produce only the necessary components to be dropped into other Calyx programs.

## Installation

Install the [calyx-py](../calyx-py.md) library.

## Command Line Options

The command line options configure the degree (or order) of the taylor series, bit width, and integer bit width.

- `--degree`: The degree of the Taylor polynomial.
- `--width`: The bit width of the value `x`.
- `--int_width`: The integer width of the value `x`. The fractional width is then inferred as `width - int_width`.
