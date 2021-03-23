from random import randint
from fud.stages.verilator.fixed_point import decimal_to_fp, fp_to_decimal, negate_twos_complement
from hypothesis import given, strategies as st


@given(bits=st.lists(st.booleans(), min_size=2, max_size=256), is_signed=st.booleans())
def verify_fixed_point_round_trip(bits, is_signed):
    """Given a bit string representation of an integer,
    selects a pseudorandom `int_width` in the
    interval [1, N - 1], and round trips the value
    through the fixed point parsing."""
    width = len(bits)
    int_width = randint(1, width - 1)

    bitstring = "".join(["1" if x else "0" for x in bits])

    def fp_round_trip(bits: str) -> str:
        # Round-trips the fixed point
        # conversion.
        return decimal_to_fp(
            fp_to_decimal(bits, width, int_width, is_signed),
            width,
            int_width,
            is_signed,
        )

    expected = int(bitstring, 2)
    actual = fp_round_trip(bitstring)
    assert (
        expected == actual
    ), f"""width: {width}, int_width:{int_width}
        is_signed: {is_signed}, bits: {bitstring}
        expected: {expected}, actual: {actual}"""


@given(bits=st.lists(st.booleans(), min_size=2, max_size=256))
def verify_twos_complement_negation_round_trip(bits):
    """Verifies that the negation of the negation
    in twos complement is the original bitstring."""
    bitstring = "".join(["1" if x else "0" for x in bits])
    width = len(bitstring)

    def round_trip(bits):
        # Round-trips the twos complement
        # negation, i.e.
        # negate(negate(bits))
        return negate_twos_complement(negate_twos_complement(bits, width), width)

    assert bitstring == round_trip(
        bitstring
    ), f"""original: {bitstring},
        round-tripped: {round_trip(bitstring)},
        width: {width}"""


if __name__ == "__main__":
    verify_fixed_point_round_trip()
    verify_twos_complement_negation_round_trip()
