from random import randint
from fud.stages.verilator.numeric_types import FixedPoint, Bitnum
from hypothesis import given, strategies as st
from math import ceil
import numpy as np


@given(bits=st.lists(st.booleans(), min_size=2, max_size=256), is_signed=st.booleans())
def test_fixed_point_round_trip(bits, is_signed):
    """Given a bit string representation of an integer,
    selects a pseudorandom `int_width` in the
    interval [1, N - 1], and round trips the value
    through the fixed point parsing."""
    width = len(bits)
    int_width = randint(1, width - 1)

    bit_string = ''.join(['1' if x else '0' for x in bits])
    base10 = int(bit_string, 2)
    hex_string = np.base_repr(base10, 16, ceil(width / 4))

    def fp_round_trip(bit_string: str) -> int:
        # Round-trips the fixed point conversion.
        value = FixedPoint(f"0b{bit_string}", width, int_width, is_signed).str_value()
        return FixedPoint(
            value,
            width,
            int_width,
            is_signed,
        )

    round_trip = fp_round_trip(bit_string)
    assert all(
        (
            bit_string == round_trip.bit_string(with_prefix=False),
            hex_string == round_trip.hex_string(with_prefix=False),
            base10 == round_trip.base10()
        )
    ), f"""width: {width}, int_width:{int_width}
        is_signed: {is_signed}, bits: {bit_string}
        base 2: {bit_string} versus {round_trip.bit_string(with_prefix=False)}
        base 16: {hex_string} versus {round_trip.hex_string(with_prefix=False)}
        base 10: {base10} versus {round_trip.base10()}"""

@given(bits=st.lists(st.booleans(), min_size=2, max_size=256), is_signed=st.booleans())
def test_bitnum_round_trip(bits, is_signed):
    """Given a bit string representation of an integer,
    round trips the value through bitnum parsing."""
    width = len(bits)

    bit_string = ''.join(['1' if x else '0' for x in bits])
    base10 = int(bit_string, 2)
    hex_string = np.base_repr(base10, 16, ceil(width / 4))

    def bitnum_round_trip(bit_string: str) -> int:
        # Round-trips the bitnum conversion.
        value = Bitnum(f"0b{bit_string}", width, is_signed).str_value()
        return Bitnum(
            value,
            width,
            is_signed
        )

    round_trip = bitnum_round_trip(bit_string)
    assert all(
        (
            bit_string == round_trip.bit_string(with_prefix=False),
            hex_string == round_trip.hex_string(with_prefix=False),
            base10 == round_trip.base10()
        )
    ), f"""width: {width}, is_signed: {is_signed}, bits: {bit_string}
        base 2: {bit_string} versus {round_trip.bit_string(with_prefix=False)}
        base 16: {hex_string} versus {round_trip.hex_string(with_prefix=False)}
        base 10: {base10} versus {round_trip.base10()}"""

