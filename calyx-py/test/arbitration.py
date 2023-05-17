# pylint: disable=import-error
import calyx.builder as cb
from calyx.py_ast import Stdlib, CompPort, CompVar


def add_i_eq_0(comp):
    """Adds wiring to check `i == 0`."""
    eq = comp.cell("eq0", Stdlib.op("eq", 32, signed=False))
    with comp.group("if_guard_0") as guard0:
        eq.left = CompPort(CompVar("i"), "out")
        eq.right = cb.const(32, 0)
        guard0.done = eq.out


def add_i_eq_1(comp):
    """Adds wiring to check `i == 1`."""
    eq = comp.cell("eq1", Stdlib.op("eq", 32, signed=False))
    with comp.group("if_guard_1") as guard1:
        eq.left = CompPort(CompVar("i"), "out")
        eq.right = cb.const(32, 1)
        guard1.done = eq.out


def add_wrap(prog):
    """Inserts the wrap component into the program.

    It has:
    - two inputs, `i` and `j`
    - two ref memories, `mem1` and `mem2`
    - one output, `out`

    For now, assume 0 <= i < 2 and 0 <= j < 4.
    if i = 0, then out = mem1[j]
    if i = 1, then out = mem2[j]
    """

    main = prog.component("wrap")
    main.input("i", 32)
    main.input("j", 32)
    main.output("out", 32)

    _ = main.mem_d1("mem1", 32, 4, 32, is_ref=True)
    _ = main.mem_d1("mem2", 32, 4, 32, is_ref=True)

    add_i_eq_0(main)
    add_i_eq_1(main)


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    add_wrap(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
