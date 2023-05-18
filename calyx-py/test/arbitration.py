# pylint: disable=import-error
from calyx.py_ast import Stdlib, CompPort, CompVar, ParComp, Enable, If
import calyx.builder as cb

# Not a big deal, but it occurs to me that a nice goal for the builder
# could be to introduce enough wrapping that we don't need to
# import anything from calyx.py_ast.


def add_i_eq_0(comp):
    """Adds wiring to check `i == 0`."""
    eq_cell = comp.cell("eq0", Stdlib.op("eq", 32, signed=False))
    with comp.comb_group("i_eq_0"):
        eq_cell.left = CompVar("i").port("out")
        eq_cell.right = cb.const(32, 0)


def add_i_eq_1(comp):
    """Adds wiring to check `i == 1`."""
    eq_cell = comp.cell("eq1", Stdlib.op("eq", 32, signed=False))
    with comp.comb_group("i_eq_1"):
        eq_cell.left = CompPort(CompVar("i"), "out")
        eq_cell.right = cb.const(32, 1)


def add_wrap(prog):
    """Inserts the component `wrap` into the program.

    It has:
    - two inputs, `i` and `j`
    - two ref memories, `mem1` and `mem2`
    - one output, `out`

    For now, assume 0 <= i < 2 and 0 <= j < 4.
    if i = 0, then out = mem1[j]
    if i = 1, then out = mem2[j]
    """

    wrap = prog.component("wrap")
    wrap.input("i", 32)
    wrap.input("j", 32)
    wrap.output("out", 32)

    _ = wrap.mem_d1("mem1", 32, 4, 32, is_ref=True)
    _ = wrap.mem_d1("mem2", 32, 4, 32, is_ref=True)

    add_i_eq_0(wrap)
    add_i_eq_1(wrap)

    # Dream: I'd like to generate these with the builder.
    # I'm running into trouble with the `port` field, which must be a guard.
    # I don't think this is a bug in the builder, but a feature:
    # the fact that I cannot do it using the builder interface
    # suggests that what I have below is actually buggy.
    # Any help is much appreciated!
    wrap.control = ParComp(
        [
            If(
                port=CompPort(CompVar("eq0"), "out"),
                cond=CompVar("i_eq_0"),
                true_branch=Enable("emit_from_mem1"),
            ),
            If(
                port=CompPort(CompVar("eq1"), "out"),
                cond=CompVar("i_eq_1"),
                true_branch=Enable("emit_from_mem2"),
            ),
        ]
    )
    # For now I've punted on actually emitting fom mem1/mem2.


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    main = prog.component("main")
    add_wrap(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
