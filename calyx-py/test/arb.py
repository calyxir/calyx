# pylint: disable=import-error
from calyx.py_ast import Stdlib, CompPort, CompVar, ParComp, Enable, If
import calyx.builder as cb

# AM, quality of life:
# It occurs to me that a nice goal for the builder library folks
# could be to introduce enough functionality that we don't need to
# import anything from calyx.py_ast.


def add_eq(comp, port_name, const, cellname, groupname):
    """Adds wiring to check `port_name == const`,
    where `port_name` is a port and `const` is an integer constant."""
    eq_cell = comp.cell(cellname, Stdlib.op("eq", 32, signed=False))
    with comp.comb_group(groupname):
        eq_cell.left = CompPort(CompVar(port_name), "out")
        # AM, point of failure:
        # This is wrong. It renders "{port_name}.out" but I want "{port_name}".
        # The same issue occurs repeatedly, so I'm just flagging this one.
        # Whenever the generated futil has {i/j}.out, I actually just want {i/j}.
        eq_cell.right = cb.const(32, const)


def add_emit_from_mem(comp, mem, ans, suffix):
    """Adds wiring that puts mem{suffix}[j] into ans."""
    with comp.group(f"emit_from_mem{suffix}") as emit_from_mem:
        mem.addr0 = CompPort(CompVar("j"), "out")  # AM: want j, not j.out
        ans.write_en = 1
        ans.write_data = mem.read_data
        emit_from_mem.done = ans.done


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

    mem1 = wrap.mem_d1("mem1", 32, 4, 32, is_ref=True)
    mem2 = wrap.mem_d1("mem2", 32, 4, 32, is_ref=True)
    ans = wrap.mem_d1("ans", 32, 1, 32, is_ref=True)

    add_eq(wrap, "i", 0, "eq0", "i_eq_0")
    add_eq(wrap, "i", 1, "eq1", "i_eq_1")
    add_emit_from_mem(wrap, mem1, ans, "1")
    add_emit_from_mem(wrap, mem2, ans, "2")

    # AM, quality of life:
    # I'd like to generate these `if` statements with the builder.
    # I tried:
    #   _: If = cb.if_(
    #     port=CompPort(CompVar("eq0"), "out"),
    #     cond=wrap.get_group("i_eq_0"),
    #     body=Enable("emit_from_mem1"),
    #   )
    # but I'm running into trouble with the `port` field, which must be
    # an ExprBuilder. On digging it looks like that's an GuardExpr in the AST,
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

    # AM, point of failure (but a stupid hack has worked):
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

    # AM, point of failure:
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
