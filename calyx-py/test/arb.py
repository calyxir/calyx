# pylint: disable=import-error
from calyx.py_ast import Stdlib, CompPort, CompVar, ParComp, Enable, If
import calyx.builder as cb

# AM:
# It occurs to me that a nice goal for the builder library folks
# could be to introduce enough functionality that we don't need to
# import anything from calyx.py_ast.


def add_i_eq_0(comp):
    """Adds wiring to check `i == 0`."""
    eq_cell = comp.cell("eq0", Stdlib.op("eq", 32, signed=False))
    with comp.comb_group("i_eq_0"):
        eq_cell.left = CompPort(CompVar("i"), "out")
        # AM: this is wrong. It renders "i.out" but I just want the port "i".
        eq_cell.right = cb.const(32, 0)


def add_i_eq_1(comp):
    """Adds wiring to check `i == 1`."""
    eq_cell = comp.cell("eq1", Stdlib.op("eq", 32, signed=False))
    with comp.comb_group("i_eq_1"):
        eq_cell.left = CompPort(CompVar("i"), "out")
        # AM: this is wrong. It renders "i.out" but I just want the port "i".
        eq_cell.right = cb.const(32, 1)


def add_wrap(prog):
    """Inserts the component `wrap` into the program.

    It has:
    - two inputs, `i` and `j`
    - two ref memories, `mem1` and `mem2`
    - one output, `out`

    For now, assume 0 <= i < 2 and 0 <= j < 4.
    if i == 0, then out = mem1[j]
    if i == 1, then out = mem2[j]
    """

    wrap = prog.component("wrap")
    wrap.input("i", 32)
    wrap.input("j", 32)

    _ = wrap.mem_d1("mem1", 32, 4, 32, is_ref=True)
    _ = wrap.mem_d1("mem2", 32, 4, 32, is_ref=True)
    _ = wrap.mem_d1("ans", 32, 1, 32, is_ref=True)

    add_i_eq_0(wrap)
    add_i_eq_1(wrap)

    # AM: I'd like to generate these if statements with the builder:
    #   _: If = cb.if_(
    #     port=CompPort(CompVar("eq0"), "out"),
    #     cond=wrap.get_group("i_eq_0"),
    #     body=Enable("emit_from_mem1"),
    #   )
    # but I'm running into trouble with the `port` field, which must be
    # an ExprBuilder. On digging it looks like that's an ast.GuardExpr,
    # but I got stuck at that point.
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
    # For now I've punted on actually emitting a value from mem1/mem2.
    # Not necessarily looking for help re: that.


def add_main(prog):
    """Inserts the component `main` into the program.
    This will be used to `invoke` the component `wrap`.

    For now, I'd like to pass it memory cells `A` and `B` by reference,
    along with the inputs i = 1, j = 3.
    """
    main = prog.component("main")
    _ = main.mem_d1("A", 32, 4, 32, is_external=True)
    _ = main.mem_d1("B", 32, 4, 32, is_external=True)
    _ = main.mem_d1("out", 32, 1, 32, is_external=True)

    # AM:
    # I'd like to add the following to the `cells` section:
    # together = wrap();

    # The following is clearly a hilarious hack:
    _ = main.cell("together", CompVar("wrap()"))
    # (I've just put parentheses in the name of the component.)

    # I think the following is what you actually want me to do:
    # _ = main.cell("together", prog.component("wrap"))
    # But _it adds a new, blank component called wrap_ to the program.
    # I'd like for it to locate the existing component `wrap`.
    # Thoughts?

    # AM:
    # Maybe I'm missing something, but I think the builder library
    # is only targeting a subset of the `invoke` functionality.
    #   class Invoke(Control):
    #     id: CompVar
    #     in_connects: List[Tuple[str, Port]]
    #     out_connects: List[Tuple[str, Port]]
    #     ref_cells: List[Tuple[str, CompVar]] = field(default_factory=list)
    #     comb_group: Optional[CompVar] = None
    #     attributes: List[Tuple[str, int]] = field(default_factory=list)
    # As I see it, only id, in_connects, and out_connects are supported.
    kwargs = {"in_i": cb.const(32, 1), "in_j": cb.const(32, 3)}
    main.control = cb.invoke(main.get_cell("together"), **kwargs)


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    add_wrap(prog)
    add_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
