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

The `fp_pow_full` component can calculate the value of `b^x` where `b` and `x` are 
any fixed point numbers. This can be calculated by first observing: 
```
b^x = e^(ln(b^x)) = e^(x*ln(b))
```
Therefore, we just calculate `x*ln(b)`, and then we can feed the result into the `exp`  
component to get our answer. 

To calculate `ln(p)` for fixed point values `p`, we use the second order [Padé Approximant](https://en.wikipedia.org/wiki/Pad%C3%A9_approximant) of `ln(p)`. We calculated the approximant 
using [Wolfram Alpha](https://www.wolframalpha.com/input?i=+PadeApproximant%5Bln%28x%29%2C%7Bx%2C1.5%2C%7B2%2C2%7D%7D%5D+).  


The `gen_exp.py` file can generate an entire Calyx program for testing purposes.
`gen_exp.py` can generate two different types of designs, depending on the 
`base_is_e` flag: if `base_is_e` is true, then the design can only caclulate 
values for `e^x`. The main component contains memories `x` (for the input) and `ret` for the result of `e^x`. 
If `base_is_e` is false, then the design can calculate values for `b^x` for any base 
`e`. Therefore, the main component contains memories `x` (the exponent input), `b` (the base intput),
and `ret` for the result of `b^x`. 
In order to generate an example program (that can only calculate exponent values with base 
`e`), with degree `4`, bit width `32`, integer bit width `16`, and `x` interpreted as a signed value:
```
./calyx-py/calyx/gen_exp.py -d 4 -w 32 -i 16 -s true -e true 
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
- `--base_is_e`: A boolean that determines whether or not to generate 
components needed to just calculate `e^x`, or to generate components needed to 
calculate `b^x` for any base `b`. 
