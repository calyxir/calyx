import numpy as np
from math import log2
from fractions import Fraction
from dataclasses import dataclass
from decimal import Decimal, getcontext
from fud.errors import InvalidNumericType


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
    bit_string_repr: str
    hex_string_repr: str
    uint_repr: int

    def __init__(self, value: str, width: int, is_signed: bool):
        if not isinstance(value, str) or len(value) == 0:
            raise InvalidNumericType(
                f"The value: {value} of type: "
                f"{type(value)} should be a non-empty string."
            )
        if value.startswith("-") and not is_signed:
            raise InvalidNumericType(
                f"A negative value was provided: {value}, " f"and `is_signed` is False."
            )
        value = value.strip()
        self.width = width
        self.is_signed = is_signed

        if value.startswith("0b"):
            self.string_repr = str(int(value, 2))
            # Zero padding for bit string.
            self.bit_string_repr = "0" * (width - len(value)) + value[2:]

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
            self.string_repr = value

    def str_value(self) -> str:
        return self.string_repr

    def bit_string(self, with_prefix=True) -> str:
        return f"{'0b' if with_prefix else ''}{self.bit_string_repr}"

    def hex_string(self, with_prefix=True) -> str:
        return f"{'0x' if with_prefix else ''}{self.hex_string_repr}"

    def unsigned_integer(self) -> int:
        return self.uint_repr

    def pretty_print(self):
        pass


@dataclass
class Bitnum(NumericType):
    """Represents a two's complement bitnum."""

    def __init__(self, value: str, width: int, is_signed: bool):
        super().__init__(value, width, is_signed)
        integer_value = int(self.string_repr)

        if all(x not in value for x in ["0x", "0b"]):
            # The actual value was passed instead of a base string representation.
            self.bit_string_repr = np.binary_repr(integer_value, self.width)
            self.uint_repr = int(self.bit_string_repr, 2)
            self.hex_string_repr = np.base_repr(self.uint_repr, 16)

        if is_signed and self.uint_repr > (2 ** (width - 1)):
            self.string_repr = str(-1 * ((2 ** width) - self.uint_repr))

        if len(self.bit_string_repr) > width:
            raise InvalidNumericType(
                f"The value: {value} will overflow when trying to represent"
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


@dataclass
class FixedPoint(NumericType):
    """Represents a fixed point number. In addition
    to the value, width, and signed-ness, it also has
    the parameter `int_width`: The integer width of
    the fixed point number. The fractional width is
    then inferred as `width - int_width`."""

    int_width: int
    frac_width: int
    decimal_repr: Decimal
    rational_repr: Fraction

    def __init__(self, value: str, width: int, int_width: int, is_signed: bool):
        super().__init__(value, width, is_signed)
        self.int_width = int_width
        self.frac_width = width - int_width
        if int_width > width:
            raise InvalidNumericType(
                f"width: {width} should be greater than the integer width: {int_width}."
            )

        if value.startswith("0b") or value.startswith("0x"):
            self.__initialize_with_base_string()
        else:
            self.__initialize(value)

    def decimal(self) -> Decimal:
        return self.decimal_repr

    def rational(self) -> Fraction:
        return self.rational_repr

    def __initialize(self, value: str):
        """Given the actual `value`, initialize
        the other values by splitting `value`
        into its integer and fractional
        representations."""
        self.string_repr = value
        self.decimal_repr = Decimal(value)
        self.rational_repr = Fraction(value)

        if self.decimal_repr == Decimal(0.0):
            self.bit_string_repr = "0" * self.width
            self.hex_string_repr = "0"
            self.uint_repr = 0
            return

        is_negative = value.startswith("-") and self.is_signed
        if is_negative:
            self.decimal_repr *= -1
            self.rational_repr *= -1

        def partition_representations():
            if self.rational_repr.denominator == 1:
                # It is a whole number.
                return int(self.decimal_repr), 0
            elif self.rational_repr < 1:
                return 0, self.decimal_repr
            else:
                ipart, fpart = str(self.decimal_repr).split(".")
                return int(ipart), "0.{}".format(fpart)

        int_partition, frac_partition = partition_representations()
        frac_width_rational = Fraction(frac_partition)

        required_frac_width = log2(frac_width_rational.denominator)
        if not required_frac_width.is_integer():
            raise InvalidNumericType(
                f"The value: `{value}` is not representable in fixed point."
            )

        required_int_width = log2(int_partition) if int_partition > 0 else 0
        int_overflow = int(required_int_width) > self.int_width - 1
        frac_overflow = int(required_frac_width) > self.frac_width
        if int_overflow or frac_overflow:
            int_msg = f"<-- Required: {int(required_int_width)}" if int_overflow else ""
            frac_msg = (
                f"<-- Required: {int(required_frac_width)}" if frac_overflow else ""
            )
            raise InvalidNumericType(
                f"""Trying to represent {value} has led to overflow.
Width: {self.width}
Integer width: {self.int_width} {int_msg}
Fractional width: {self.frac_width} {frac_msg}"""
            )

        int_bits = np.binary_repr(int_partition, self.int_width)
        frac_bits = np.binary_repr(
            round(frac_width_rational * (2 ** self.frac_width)), self.frac_width
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
        is_negative = self.is_signed and (self.bit_string_repr.startswith("1"))
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
