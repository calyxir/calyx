from random import randint
from calyx.numeric_types import FixedPoint, Bitnum, InvalidNumericType
from hypothesis import given, strategies as st  # type: ignore
import numpy as np
import pytest  # type: ignore


@given(bits=st.lists(st.booleans(), min_size=2, max_size=256), is_signed=st.booleans())
def test_fixed_point_round_trip(bits, is_signed):
    """Given a bit string representation of an integer,
    selects a pseudorandom `int_width` in the
    interval [1, N - 1], and round trips the value
    through the fixed point parsing."""
    width = len(bits)
    int_width = randint(1, width - 1)

    bit_string = "".join(["1" if x else "0" for x in bits])
    unsigned_integer = int(bit_string, 2)
    hex_string = np.base_repr(unsigned_integer, 16)

    def fp_round_trip(bit_string: str) -> FixedPoint:
        # Round-trips the fixed point conversion.
        bin = FixedPoint(f"0b{bit_string}", width, int_width, is_signed).str_value()
        hex = FixedPoint(f"0x{hex_string}", width, int_width, is_signed).str_value()
        assert bin == hex
        return FixedPoint(
            bin,
            width,
            int_width,
            is_signed,
        )

    round_trip = fp_round_trip(bit_string)
    assert all(
        (
            bit_string == round_trip.bit_string(with_prefix=False),
            hex_string == round_trip.hex_string(with_prefix=False),
            unsigned_integer == round_trip.unsigned_integer(),
        )
    ), f"""width: {width}, int_width:{int_width}
        is_signed: {is_signed}, bits: {bit_string}
        base 2: {bit_string} versus {round_trip.bit_string(with_prefix=False)}
        base 16: {hex_string} versus {round_trip.hex_string(with_prefix=False)}
        base 10: {unsigned_integer} versus {round_trip.unsigned_integer()}"""


@given(bits=st.lists(st.booleans(), min_size=2, max_size=256), is_signed=st.booleans())
def test_bitnum_round_trip(bits, is_signed):
    """Given a bit string representation of an integer,
    round trips the value through bitnum parsing."""
    width = len(bits)

    bit_string = "".join(["1" if x else "0" for x in bits])
    unsigned_integer = int(bit_string, 2)
    hex_string = np.base_repr(unsigned_integer, 16)

    def bitnum_round_trip(bit_string: str) -> Bitnum:
        # Round-trips the bitnum conversion.
        bin = Bitnum(f"0b{bit_string}", width, is_signed).str_value()
        hex = Bitnum(f"0x{hex_string}", width, is_signed).str_value()
        assert bin == hex
        return Bitnum(bin, width, is_signed)

    round_trip = bitnum_round_trip(bit_string)
    assert all(
        (
            bit_string == round_trip.bit_string(with_prefix=False),
            hex_string == round_trip.hex_string(with_prefix=False),
            unsigned_integer == round_trip.unsigned_integer(),
        )
    ), f"""width: {width}, is_signed: {is_signed}, bits: {bit_string}
        base 2: {bit_string} versus {round_trip.bit_string(with_prefix=False)}
        base 16: {hex_string} versus {round_trip.hex_string(with_prefix=False)}
        base 10: {unsigned_integer} versus {round_trip.unsigned_integer()}"""


def test_fp_int_width_overflow():
    with pytest.raises(InvalidNumericType, match=r"overflow"):
        FixedPoint("2.0", 2, 1, False)
    with pytest.raises(InvalidNumericType, match=r"overflow"):
        FixedPoint("0b101", 2, 1, True)


def test_fp_frac_width_overflow():
    with pytest.raises(InvalidNumericType, match=r"overflow"):
        FixedPoint("0.25", 2, 1, False)
    with pytest.raises(InvalidNumericType, match=r"overflow"):
        FixedPoint("0b111", 2, 1, True)
    with pytest.raises(InvalidNumericType, match=r"overflow"):
        FixedPoint("0x7", 2, 1, False)


def test_bitnum_overflow():
    with pytest.raises(InvalidNumericType, match=r"overflow"):
        Bitnum("0b10", 1, False)
    with pytest.raises(InvalidNumericType, match=r"overflow"):
        Bitnum("-2", 1, True)
    with pytest.raises(InvalidNumericType, match=r"overflow"):
        Bitnum("0x2", 1, False)


def test_fp_invalid_representation():
    with pytest.raises(InvalidNumericType, match=r"not representable"):
        FixedPoint("0.1", 2, 1, False)


def test_unsigned_negative_number():
    with pytest.raises(InvalidNumericType, match=r"negative value"):
        FixedPoint("-0.5", 2, 1, False)
    with pytest.raises(InvalidNumericType, match=r"negative value"):
        Bitnum("-1", 2, False)


def test_empty_string():
    with pytest.raises(InvalidNumericType, match=r"non-empty string"):
        Bitnum("", 2, False)
    with pytest.raises(InvalidNumericType, match=r"non-empty string"):
        FixedPoint("", 2, 1, False)


def test_non_string_initialization():
    with pytest.raises(InvalidNumericType, match=r"string"):
        Bitnum(16, 5, False)
    with pytest.raises(InvalidNumericType, match=r"string"):
        FixedPoint(0.5, 2, 1, False)
