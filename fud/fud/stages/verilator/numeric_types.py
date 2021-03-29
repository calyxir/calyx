import numpy as np
from math import log2, ceil
from fractions import Fraction
from dataclasses import dataclass
from decimal import Decimal, getcontext


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
    base10_repr: int
    hex_string_padding: int  # Padding used for hex string representation.

    def __init__(self, value: str, width: int, is_signed: bool):
        assert isinstance(value, str), f"value: {value} should be in string form."
        assert len(value) > 0, f"Empty string passed in."

        if value[0] == "-":
            assert (
                is_signed
            ), f"Negative value: {value} needs `is_signed` to be set to True."
        value = value.strip()
        self.width = width
        self.is_signed = is_signed
        self.hex_string_padding = ceil(self.width / 4)

        if "0b" in value:
            self.string_repr = str(int(value, 2))
            difference = width - len(value)  # Zero padding.
            self.bit_string_repr = "0" * difference + value[2:]
            self.base10_repr = int(self.bit_string_repr, 2)
            self.hex_string_repr = np.base_repr(
                self.base10_repr, 16, self.hex_string_padding
            )
        elif "0x" in value:
            self.string_repr = str(int(value, 16))
            self.hex_string_repr = value[2:]
            self.bit_string_repr = np.binary_repr(
                int(self.hex_string_repr, 16), self.width
            )
            self.base10_repr = int(self.bit_string_repr, 2)
        else:
            self.string_repr = value

    def str_value(self) -> str:
        return self.string_repr

    def bit_string(self, with_prefix=True) -> str:
        return f"{'0b' if with_prefix else ''}{self.bit_string_repr}"

    def hex_string(self, with_prefix=True) -> str:
        return f"{'0x' if with_prefix else ''}{self.hex_string_repr}"

    def base10(self) -> int:
        return self.base10_repr

    def pretty_print(self):
        pass


@dataclass
class Bitnum(NumericType):
    """Represents a two's complement bitnum."""
    def __init__(self, value: str, width: int, is_signed: bool):
        super().__init__(value, width, is_signed)
        unsigned_value = int(self.string_repr)
        if all(x not in value for x in ["0x", "0b"]):
            # The actual value was passed instead of a base string representation.
            self.bit_string_repr = np.binary_repr(unsigned_value, self.width)
            self.base10_repr = int(self.bit_string_repr, 2)
            self.hex_string_repr = np.base_repr(
                self.base10_repr, 16, self.hex_string_padding
            )

        if is_signed and self.base10_repr > (2 ** (width - 1)):
            self.string_repr = str(-1 * ((2 ** width) - self.base10_repr))

    def pretty_print(self):
        print(
            f"""
            {'Signed' if self.is_signed else ''} Bitnum: {self.string_repr} 
            ------------------
            Width: {self.width}
            Bit String: 0b{self.bit_string_repr}
            Hex String: 0x{self.hex_string_repr}
            Base 10: {self.base10_repr}"""
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
        assert (
            width > int_width
        ), f"width: {width} should be greater than the integer width: {int_width}."

        if any(x in value for x in ["0x", "0b"]):
            self.__initialize_with_base_string(value)
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
            self.hex_string_repr = "0" * self.hex_string_padding
            self.bit_string_repr = "0" * self.width
            self.base10_repr = 0
            return

        is_negative = self.is_signed and value[0] == "-"
        if is_negative:
            self.decimal_repr *= -1
            self.rational_repr *= -1

        def split_bit_types():
            # Splits the number into its integer
            # and fractional parts.
            if self.rational_repr.denominator == 1:
                # It is a whole number.
                return self.decimal_repr, 0

            if self.rational_repr < 1:
                # This is necessary because the string version
                # of an egregiously small number may include a
                # period for scientific notation, e.g. `4.2e-20`.
                return 0, self.decimal_repr

            integer_part, fractional_part = str(self.decimal_repr).split(".")
            return integer_part, Decimal(f"0.{fractional_part}")

        integer_part, fractional_part = split_bit_types()

        int_bits = np.binary_repr(int(integer_part), width=self.int_width)
        frac_width_rational = Fraction(fractional_part)
        frac_bits = np.binary_repr(
            round(frac_width_rational * (2 ** self.frac_width)), self.frac_width
        )

        num_int_bits = len(int_bits)
        int_overflow = num_int_bits > self.int_width
        required_width = log2(frac_width_rational.denominator)
        if not required_width.is_integer():
            raise Exception(
                f"The value: `{value}` is not representable in fixed point."
            )

        frac_overflow = int(required_width) > self.frac_width
        # Verify no integer or fractional width overflow in representation.
        if int_overflow or frac_overflow:
            int_msg = f"<-- Required: {num_int_bits}" if int_overflow else ""
            frac_msg = f"<-- Required: {int(required_width)}" if frac_overflow else ""
            raise Exception(
                f"""Trying to represent {self.decimal_repr} has led to overflow.
                Width: {self.width}
                Integer width: {self.int_width} {int_msg}  
                Fractional width: {self.frac_width} {frac_msg}"""
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
        self.base10_repr = int(bits, 2)
        self.hex_string_repr = np.base_repr(
            self.base10_repr, 16, self.hex_string_padding
        )

    def __initialize_with_base_string(self, value: str):
        """Initializes the value given the the bit string."""
        is_negative = self.is_signed and (self.bit_string_repr[0] == "1")
        if is_negative:
            # Negate it to calculate the positive value.
            self.bit_string_repr = self.__negate_twos_complement(self.bit_string_repr)

        # Determine expected value by summing over
        # the binary values of the given bits.
        # Integer exponential value begins at int_width - 1.
        exponent = 2 ** (self.int_width - 1)

        integer_value = 0
        # Sum over integer bits.
        for i in range(0, self.int_width):
            if self.bit_string_repr[i] == "1":
                integer_value += exponent
            exponent >>= 1

        # The fractional bits begin at value 1/2.
        denominator = 2

        fractional_value = Fraction(0)
        # Sum over fractional bits.
        for i in range(self.int_width, self.width):
            if self.bit_string_repr[i] == "1":
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
        if is_negative:
            # Re-negate the two's complement form.
            self.bit_string_repr = self.__negate_twos_complement(self.bit_string_repr)
            self.decimal_repr = value * Decimal(-1)
        else:
            self.decimal_repr = value

        self.string_repr = str(self.decimal_repr)
        self.rational_repr = Fraction(self.decimal_repr)

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
            f"""
            {'Signed' if self.is_signed else ''} Fixed Point: {self.string_repr} 
            ------------------
            Width: {self.width}, IntWidth: {self.int_width}, FracWidth: {self.frac_width}
            Decimal Class: {self.decimal_repr}
            Fraction Class: {self.rational_repr}
            Bit String: 0b{self.bit_string_repr}
            Hex String: 0x{self.hex_string_repr}
            Base 10: {self.base10_repr}"""
        )
