from itertools import product
from decimal import Decimal
from calyx.numeric_types import FixedPoint


def compute_exp_frac_table(frac_width: int):
    """Computes a table of size 2^frac_width
    for every value of e^x that can be
    represented by fixed point in the range [0, 1].
    """
    # Chebyshev approximation coefficients for e^x in [0, 1].
    # Credits to J. Sach's blogpost here:
    # https://www.embeddedrelated.com/showarticle/152.php
    coeffs = [
        Decimal("1.7534"),
        Decimal("0.85039"),
        Decimal("0.10521"),
        Decimal("0.0087221"),
        Decimal("0.00054344"),
        Decimal("0.000027075"),
    ]

    def chebyshev_polynomial_approx(x):
        """Computes the Chebyshev polynomials
        based on the recurrence relation
        described here:
        en.wikipedia.org/wiki/Chebyshev_polynomials#Definition
        """
        # Change from [0, 1] to [-1, 1] for
        # better approximation with chebyshev.
        u = Decimal(2 * x - 1)

        Ti = Decimal(1)
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
    binary_permutations = map(lambda l: list(l), product(["0", "1"], repeat=frac_width))

    e_table = [0] * (2**frac_width)
    for permutation in binary_permutations:
        i = int("".join(permutation), 2)
        fraction = Decimal(i / 2 ** (frac_width))
        e_table[i] = chebyshev_polynomial_approx(fraction)

    return e_table


def exp(x: str, width: int, int_width: int, is_signed: bool, print_results=False):
    """
    Computes an approximation of e^x.
    This is done by splitting the fixed point number
    x into its integral bits `i` and fractional bits `f`,
    and computing e^(i + f) = e^i * e^f.
    For the fractional portion, a chebyshev
    approximation is used.

    Example:
        exp(
            x="1.0",
            width=32,
            int_width=16,
            is_signed=True,
            print_results=True
        ) # Should return an approximation of e^(1.0)
    """
    frac_width = width - int_width
    bin_string = FixedPoint(x, width, int_width, is_signed=False).bit_string(
        with_prefix=False
    )

    int_b = bin_string[:int_width]
    int_bin = int(int_b, 2)
    frac_b = bin_string[int_width:width]
    frac_bin = int(frac_b, 2)

    # Split e^x into e^i * e^f.
    e_i = Decimal("2.71828") ** int_bin

    e_table = compute_exp_frac_table(frac_width)
    e_f = e_table[frac_bin]

    # Compute e^i * e^f.
    actual = Decimal(e_i) * Decimal(e_f)

    if print_results:
        accepted = Decimal("2.71828") ** Decimal(x)
        print(
            f"e^{x}: {accepted}, actual: {actual}"
            f"relative difference: {(actual - accepted) / actual * 100} %"
        )

    return actual
