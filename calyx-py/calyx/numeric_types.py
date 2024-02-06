import base64
from typing import Tuple
import numpy as np
from math import log2
from fractions import Fraction
from dataclasses import dataclass
from decimal import Decimal, getcontext
import math
import logging as log


class InvalidNumericType(Exception):
    """
    An error raised when an invalid numeric type is provided.
    """

    def __init__(self, msg):
        msg = f"""Invalid Numeric Type: {msg}"""
        super().__init__(msg)


@dataclass
class NumericType:
    """Interface for a numeric type.
    The following are required to be passed in by the user:
    1. `value`: The value of the number. It may come in 3 forms:
                (a) The actual value, e.g. `42`
                (b) Binary string, e.g. `0b1101`
                (c) Hexadecimal string, e.g. `0xFF`
    2. `width`: The bit width of the entire number.
    4. `is_signed`: The signed-ness of the number."""

    width: int
    is_signed: bool
    string_repr: str
    is_undef: bool = False
    bit_string_repr: str = None
    hex_string_repr: str = None
    uint_repr: int = None

    def __init__(self, value: str, width: int, is_signed: bool):
        if not isinstance(value, str) or len(value) == 0:
            raise InvalidNumericType(
                f"The value: {value} of type: "
                f"{type(value)} should be a non-empty string."
            )
        elif value.startswith("-") and not is_signed:
            raise InvalidNumericType(
                f"A negative value was provided: {value}, and `is_signed` is False."
            )
        # Some backends may use `x` to represent an uninitialized digit, e.g. `0bxxxx`.
        # Since this cannot be properly translated into a number, returns error.
        value = value.strip()
        self.width = width
        self.is_signed = is_signed

        stripped_prefix = value[2:] if value.startswith("0x") else value
        if any(digit == "x" for digit in stripped_prefix):
            log.error(
                f"Memory contains the value: `{value}', which is uninitialized. "
                "This happens when the Calyx design attempts to read a port"
                " that is not connected to anything."
                " Try to generate a VCD file using `--to vcd` and look for signals "
                "that are not driven in a given cycle."
            )
            self.string_repr = str(stripped_prefix)
            self.is_undef = True
        elif value.startswith("0b"):
            self.string_repr = str(int(value, 2))
            # Zero padding for bit string.
            self.bit_string_repr = "0" * max(width - len(value), 0) + value[2:]

            self.uint_repr = int(self.bit_string_repr, 2)
            self.hex_string_repr = np.base_repr(self.uint_repr, 16)
        elif value.startswith("0x"):
            self.string_repr = str(int(value, 16))
            self.hex_string_repr = value[2:]
            self.bit_string_repr = np.binary_repr(
                int(self.hex_string_repr, 16), self.width
            )
            self.uint_repr = int(self.bit_string_repr, 2)
        else:
            # The decimal representation was passed in.
            self.string_repr = value

    def str_value(self) -> str:
        return self.string_repr

    def bit_string(self, with_prefix=True) -> str:
        return f"{'0b' if with_prefix else ''}{self.bit_string_repr}"

    def hex_string(self, with_prefix=True) -> str:
        return f"{'0x' if with_prefix else ''}{self.hex_string_repr}"

    def unsigned_integer(self) -> int:
        return self.uint_repr

    def base_64_encode(self) -> bytes:
        return base64.standard_b64encode(
            self.uint_repr.to_bytes(math.ceil(self.width / 8), "little")
        )

    def pretty_print(self):
        pass


@dataclass
class Bitnum(NumericType):
    """Represents a two's complement bitnum."""

    def __init__(self, value: str, width: int, is_signed: bool):
        super().__init__(value, width, is_signed)

        if self.is_undef:
            return
        if self.bit_string_repr is None and self.hex_string_repr is None:
            # The decimal representation was passed in.
            self.bit_string_repr = np.binary_repr(int(self.string_repr), self.width)
            self.uint_repr = int(self.bit_string_repr, 2)
            self.hex_string_repr = np.base_repr(self.uint_repr, 16)

        if is_signed and self.uint_repr > (2 ** (width - 1)):
            negated_value = -1 * ((2**width) - self.uint_repr)
            self.string_repr = str(negated_value)

        if len(self.bit_string_repr) > width:
            raise InvalidNumericType(
                f"The value: {value} will overflow when trying to represent "
                f"{len(self.bit_string_repr)} bits with width: {width}"
            )

    def pretty_print(self):
        print(
            f"""{'Signed' if self.is_signed else ''} Bitnum: {self.string_repr}
------------------
Width: {self.width}
Bit String: 0b{self.bit_string_repr}
Hex String: 0x{self.hex_string_repr}
Unsigned Integer: {self.uint_repr}"""
        )


def partition(decimal: Decimal, rational: Fraction) -> Tuple[int, Fraction]:
    if rational.denominator == 1:
        # It is a whole number.
        return int(decimal), Fraction(0)
    elif rational < 1:
        # Catches the scientific notation case,
        # e.g. `4.2e-20`.
        return 0, Fraction(decimal)
    else:
        ipart, fpart = str(decimal).split(".")
        return int(ipart), Fraction("0.{}".format(fpart))


@dataclass
class FixedPoint(NumericType):
    """Represents a fixed point number. In addition
    to the value, width, and signed-ness, it also has
    the parameter `int_width`: The integer width of
    the fixed point number. The fractional width is
    then inferred as `width - int_width`."""

    int_width: int = None
    frac_width: int = None
    decimal_repr: Decimal = None
    rational_repr: Fraction = None

    def __init__(self, value: str, width: int, int_width: int, is_signed: bool):
        super().__init__(value, width, is_signed)
        self.int_width = int_width
        self.frac_width = width - int_width
        if int_width > width:
            raise InvalidNumericType(
                f"width: {width} should be greater than the integer width: {int_width}."
            )

        if self.bit_string_repr is not None or self.hex_string_repr is not None:
            self.__initialize_with_base_string()
        else:
            self.__initialize_with_decimal_repr(value)

    def decimal(self) -> Decimal:
        return self.decimal_repr

    def rational(self) -> Fraction:
        return self.rational_repr

    def __initialize_with_decimal_repr(self, value: str):
        """Given the decimal representation,
        initialize the other values by splitting
        `value` into its integer and fractional
        representations."""
        self.string_repr = value
        self.decimal_repr = Decimal(value)
        self.rational_repr = Fraction(value)

        is_negative = self.is_signed and value.startswith("-")
        if is_negative:
            self.decimal_repr *= -1
            self.rational_repr *= -1

        int_partition, frac_partition = partition(self.decimal_repr, self.rational_repr)

        required_frac_width = log2(frac_partition.denominator)
        if not required_frac_width.is_integer():
            raise InvalidNumericType(
                f"The value: `{value}` is not representable in fixed point."
            )
        required_int_width = int(log2(int_partition)) + 1 if int_partition > 0 else 0
        required_frac_width = int(required_frac_width)
        int_overflow = required_int_width > self.int_width
        frac_overflow = required_frac_width > self.frac_width
        if int_overflow or frac_overflow:
            raise InvalidNumericType(
                f"""Trying to represent {value} with:
    Integer width: {self.int_width}
    Fractional width: {self.frac_width}
has led to overflow.
{'Required int width: {}'.format(required_int_width) if int_overflow else ''}
{'Required fractional width: {}'.format(required_frac_width) if frac_overflow else ''}
"""
            )

        int_bits = np.binary_repr(int_partition, self.int_width)
        frac_bits = (
            np.binary_repr(
                round(frac_partition * (2**self.frac_width)), self.frac_width
            )
            if self.frac_width > 0
            else ""
        )
        # Given the binary form of the integer part and fractional part of
        # the decimal, simply append the two strings.
        bits = int_bits + frac_bits

        if is_negative:
            # Re-negate the decimal representation.
            self.decimal_repr *= -1
            self.rational_repr *= -1
            bits = self.__negate_twos_complement(bits)

        self.bit_string_repr = bits
        self.uint_repr = int(bits, 2)
        self.hex_string_repr = np.base_repr(self.uint_repr, 16)

    def __initialize_with_base_string(self):
        """Initializes the value given the bit string."""
        if len(self.bit_string_repr) > self.width:
            raise InvalidNumericType(
                f"The bit string: {self.bit_string_repr} will "
                f"overflow when trying to represent {len(self.bit_string_repr)} "
                f"bits with width: {self.width}."
            )
        is_negative = self.is_signed and self.bit_string_repr.startswith("1")
        if is_negative:
            # Negate it to calculate the positive value.
            self.bit_string_repr = self.__negate_twos_complement(self.bit_string_repr)

        # Determine expected value by summing over
        # the binary values of the given bits.
        exponent = 2 ** (self.int_width - 1)

        int_value = 0
        # Sum over integer bits.
        for i in range(0, self.int_width):
            if self.bit_string_repr[i] == "1":
                int_value += exponent
            exponent >>= 1

        # The fractional bits begin at value 1/2.
        denominator = 2

        frac_value = Fraction(0)
        # Sum over fractional bits.
        for i in range(self.int_width, self.width):
            if self.bit_string_repr[i] == "1":
                frac_value += Fraction(1, denominator)
            denominator <<= 1

        # Set the Decimal precision to ensure small fractional
        # values are still represented precisely.
        getcontext().prec = 64
        frac_value = Decimal(frac_value.numerator / frac_value.denominator)

        if is_negative:
            # Re-negate the two's complement form.
            self.bit_string_repr = self.__negate_twos_complement(self.bit_string_repr)
            self.decimal_repr = Decimal(int_value + frac_value) * Decimal(-1)
        else:
            self.decimal_repr = Decimal(int_value + frac_value)

        self.rational_repr = Fraction(self.decimal_repr)
        self.string_repr = str(self.decimal_repr)

    def __negate_twos_complement(self, bitstring: str) -> str:
        """Takes in a bit string and returns the negated
        form in two's complement. This is done by:
        (1) Flipping the bits
        (2) Adding 1.
        Example:
            negative_twos_complement(
                bitstring="011"
            )
            = "101"
            = `-3` in two's complement."""
        if all(b == "0" for b in bitstring):
            # Two's complement of zero is itself.
            return bitstring

        # Flip bits.
        bitstring = "".join(["1" if b == "0" else "0" for b in bitstring])

        width = len(bitstring)
        # Add one.
        return np.binary_repr(int(bitstring, 2) + 1, width)

    def pretty_print(self):
        print(
            f"""{'Signed' if self.is_signed else ''} Fixed Point: {self.string_repr}
------------------
Width: {self.width}, IntWidth: {self.int_width}, FracWidth: {self.frac_width}
Decimal Class: {self.decimal_repr}
Fraction Class: {self.rational_repr}
Bit String: 0b{self.bit_string_repr}
Hex String: 0x{self.hex_string_repr}
Unsigned Integer: {self.uint_repr}"""
        )


def bitnum_to_fixed(bitnum: Bitnum, int_width: int) -> FixedPoint:
    return FixedPoint(
        value="0b" + bitnum.bit_string_repr,
        width=bitnum.width,
        int_width=int_width,
        is_signed=bitnum.is_signed,
    )
