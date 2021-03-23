import numpy as np
from math import log2
from fractions import Fraction
from decimal import Decimal, getcontext


def binary_to_base10(bitstring: str) -> int:
    """Takes a binary number in string form
    e.g. "1010" and returns the
    corresponding base 10 number.
    """
    out = 0
    for bit in bitstring:
        out = (out << 1) | int(bit)
    return out


def negate_twos_complement(bitstring: str, width: int) -> str:
    """Takes in a bit string and returns the negated
    form in two's complement. This is done by:
    (1) Flipping the bits
    (2) Adding 1.
    Example:
        negative_twos_complement(
            bitstring="011",
            width=3
        )
        = "101"
        = `-3` in two's complement."""
    length = len(bitstring)
    assert (
        length == width
    ), f"bitstring: {bitstring} does not have width: {width}. Actual: {length}."

    if all(b == "0" for b in bitstring):
        # Two's complement of zero is itself.
        return bitstring

    # Flip bits.
    bitstring = "".join(["1" if b == "0" else "0" for b in bitstring])

    # Add one.
    return np.binary_repr(int(bitstring, 2) + 1, width)


def fp_to_decimal(bits: str, width: int, int_width: int, is_signed: bool) -> Decimal:
    """Takes in a fixed point number in bit string
    representation with the bit width, integer bit
    width, and signed-ness, and returns the Decimal
    value.

    Note: Assumes the fixed point number is in
    two's complement form."""
    assert len(bits) > 0, "The empty bit string cannot be converted to decimal."

    is_negative = is_signed and (bits[0] == "1")
    if is_negative:
        # Negate it to calculate the positive value.
        bits = negate_twos_complement(bits, width)

    # Determine expected value by summing over
    # the binary values of the given bits.
    # Integer exponential value begins at int_width - 1.
    exponent = 2 ** (int_width - 1)

    integer_value = 0
    # Sum over integer bits.
    for i in range(0, int_width):
        if bits[i] == "1":
            integer_value += exponent
        exponent >>= 1

    # The fractional bits begin at value 1/2.
    denominator = 2

    fractional_value = Fraction(0)
    # Sum over fractional bits.
    for i in range(int_width, width):
        if bits[i] == "1":
            fractional_value += Fraction(1, denominator)
        denominator <<= 1

    # Set the Decimal precision to ensure
    # small fractional values are still
    # represented precisely.
    getcontext().prec = 64

    fractional_value = Decimal(
        fractional_value.numerator / fractional_value.denominator
    )

    value = Decimal(integer_value + fractional_value)
    return value * Decimal(-1) if is_negative else value


# TODO(cgyurgyik): Eventually, we want to use
# truncation for values that cannot be exactly
# represented. Warning flag for user-provided
# fixed point numbers as well.
def verify_representation_in_fp(x: Decimal, width: int, int_width: int):
    """Raises an exception if the fractional value
    `x` is not representable by fixed point numbers,
    otherwise does nothing."""
    rational_x = Fraction(x)
    numerator = rational_x.numerator
    denominator = rational_x.denominator
    log2_d = log2(denominator)
    frac_width = width - int_width

    # The fractional value `x` is representable by
    # fixed point if:
    # (1) The numerator is zero or (numerator - 1) is even.
    # (2) The denominator is a power of 2 that is
    #     less than or equal to the fractional width.
    if all(
        (
            numerator == 0 or (numerator - 1) % 2 == 0,
            log2_d.is_integer() and log2_d <= frac_width,
        )
    ):
        return

    raise Exception(
        f"The fractional part of {x}: "
        f"{numerator} / {denominator} "
        f"cannot be represented as a "
        f"fixed point. [width: {width}, "
        f"integer width: {int_width}, "
        f"fractional width: {frac_width}]."
    )


def decimal_to_fp(value: Decimal, width: int, int_width: int, is_signed: bool) -> int:
    """Given the value, width, integer width and signed-ness,
    returns the fixed point representation in two's complement,
    and converts it to base 10.
    """
    if value == 0.0:
        return 0

    if not is_signed and value < 0.0:
        raise Exception(
            f"A negative value was passed in, {value}, " f"and `is_signed` is False."
        )

    is_negative = is_signed and value < 0.0
    if is_negative:
        value *= -1

    def split_bit_types(x):
        # Splits the number into its integer
        # and fractional parts.
        rational_x = Fraction(x)
        if rational_x.denominator == 1:
            # It is a whole number.
            return x, 0

        if rational_x < 1:
            # This is necessary because the string version
            # of an egregiously small number may include a
            # period for scientific notation, e.g. `4.2e-20`.
            return 0, x

        integer_part, fractional_part = str(x).split(".")
        return integer_part, Decimal(f"0.{fractional_part}")

    integer_part, fractional_part = split_bit_types(value)
    # Verifies the fractional part can be represented with powers of two.
    verify_representation_in_fp(fractional_part, width, int_width)

    int_bits = np.binary_repr(int(integer_part), width=int_width)
    frac_width = width - int_width
    frac_bits = np.binary_repr(
        int(Fraction(fractional_part) * (2 ** frac_width)), frac_width
    )

    num_int_bits = len(int_bits)
    num_frac_bits = len(frac_bits)
    # Verify no overflow in representation.
    if num_int_bits > int_width or num_frac_bits > frac_width:
        raise Exception(
            f"""Trying to represent {value} with
            Width: {width}
            Integer width: {int_width}
            Fractional width: {frac_width}
            has led to overflow.
            Required number of integer bits: {num_int_bits}.
            Required number of fractional bits: {num_frac_bits}."""
        )
    # Given the binary form of the integer part and fractional part of
    # the decimal, simply append the two strings.
    bits = int_bits + frac_bits

    if is_negative:
        bits = negate_twos_complement(bits, width)

    return binary_to_base10(bits)
