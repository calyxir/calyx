import numpy as np
from itertools import product


def decimal_to_fixed_p(num, width, int_bit, frac_bit):
    """Given the number, width, integer bit and fractional bit,
    returns the fixed point representation.
    Example: decimal_to_fixed_p(11.125,8,5,3) returns 01011001 = 2^3+2^1+2^0+2^(-3)
    Precondition: There is no overflow
    (integer part of the number should be representable with int_bit number of bits).
    """
    # separate into integer and fractional parts
    intg, frac = str(num).split(".")
    int_b = np.binary_repr(int(intg), width=int_bit)
    frac = "0." + frac

    # multiply fractional part with 2**frac_bit to turn into integer
    frac = float(frac) * float(2 ** frac_bit)
    _, f = str(frac).split(".")
    # raises Exception when the number can't
    # be represented in fixed point format.
    if f != "0":
        raise Exception("Can't be represented as fixedpoint numbers.")
    frac = int(frac)
    frac_b = np.binary_repr(frac, width=frac_bit)
    r = int_b + frac_b
    return r


def fixed_p_to_decimal(fp, width, int_bit, frac_bit):
    """Given fixedpoint representation, width,
    integer bit and fractinal bit, returns the number.
    example: fixed_p_to_decimal ('01011001',8,5,3) returns 11.125
    """
    int_b = fp[:int_bit]
    frac_b = fp[int_bit:width]
    int_num = int(int_b, 2)
    frac = int(frac_b, 2)
    frac_num = float(frac / 2 ** (frac_bit))
    num = float(int_num + frac_num)
    return num


def binary_to_base10(bit_list):
    """Takes a binary number in list form
    e.g. [1, 0, 1, 0], and returns
    the corresponding base 10 number.
    """
    out = 0
    for b in bit_list:
        out = (out << 1) | b
    return out


def compute_exp_frac_table(frac_bit):
    """Computes a table of size 2^frac_bit
    for every value of e^x that can be
    represented by fixed point in the range [0, 1].
    """
    # Chebyshev approximation coefficients for e^x in [0, 1].
    coeffs = [
        1.7534,
        0.85039,
        0.10521,
        0.0087221,
        0.00054344,
        0.000027075
    ]

    def chebyshev_polynomial_approx(x):
        """Computes the Chebyshev polynomials
        based on the recurrence relation
        described here:
        en.wikipedia.org/wiki/Chebyshev_polynomials#Definition
        """
        # Change from [0, 1] to [-1, 1] for
        # better approximation with chebyshev.
        u = (2 * x - 1)

        Ti = 1
        Tn = None
        T = u
        num_coeffs = len(coeffs)
        c = coeffs[0]
        for i in range(1, num_coeffs):
            c = c + T * coeffs[i]
            Tn = 2 * u * T - Ti
            Ti = T
            T = Tn

        return c

    # Gets the permutations of 2^f_bit,
    # in increasing order.
    binary_permutations = map(
        list,
        product(
            [0, 1],
            repeat=frac_bit
        )
    )

    e_table = [0] * (2 ** frac_bit)
    for p in binary_permutations:
        i = binary_to_base10(p)
        fraction = float(
            i / 2 ** (frac_bit)
        )
        e_table[i] = chebyshev_polynomial_approx(fraction)

    return e_table


def exp(x, width, int_bit, frac_bit, print_results=False):
    """
    Computes an approximation of e^x.
    This is done by splitting the fixed point number
    x into its integral bits `i` and fractional bits `f`,
    and computing e^(i + f) = e^i * e^f.

    For the fractional portion, a chebyshev
    approximation is used.
    """
    fp_x = decimal_to_fixed_p(x, width, frac_bit, int_bit)

    int_b = fp_x[:int_bit]
    int_bin = int(int_b, 2)
    frac_b = fp_x[int_bit:width]
    frac_bin = int(frac_b, 2)

    # Split e^x into e^i * e^f.
    e_i = 2.71828 ** int_bin

    e_table = compute_exp_frac_table(frac_bit)
    e_f = e_table[frac_bin]

    # Compute e^i * e^f.
    expected = e_i * e_f

    if print_results:
        actual = 2.71828 ** x
        print(f'e^{x}')
        print(f'approx: {expected}')
        print(f'actual: {actual}')
        print(f'reldef: {(expected - actual) / expected * 100}%')

    return expected
