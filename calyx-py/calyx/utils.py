import textwrap
import math


def block(decl, contents, indent=2, sep="\n"):
    """Format a block like this:
        decl {
          contents[0] <sep> contents[1] ...
        }
    where `decl` is one line and contents is a list that is separated with `sep`.
    """
    return "".join(
        (decl, " {\n", textwrap.indent(sep.join(contents), indent * " "), "\n}")
    )


def bits_needed(num):
    """
    Number of bits needed to represent `num`.
    """
    return math.floor(math.log(num, 2)) + 1


def float_to_fixed_point(value: float, N: int) -> float:
    """Returns a fixed point representation of `value`
    with the decimal value truncated to `N - 1` places.
    """
    w = 2 << (N - 1)
    return round(value * w) / float(w)
