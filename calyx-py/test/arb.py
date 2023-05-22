# pylint: disable=import-error
from calyx.py_ast import Stdlib, CompInst
import calyx.builder as cb


def add_eq(comp, port_name, const, cellname, groupname):
    """Adds wiring to check `port_name == const`,
    where `port_name` is a port and `const` is an integer constant."""
    eq_cell = comp.cell(cellname, Stdlib.op("eq", 32, signed=False))
    with comp.comb_group(groupname):
        eq_cell.left = comp.this()[port_name]
        eq_cell.right = cb.const(32, const)


def add_emit_from_mem(comp, mem, ans, suffix):
    """Adds wiring that puts mem{suffix}[j] into ans."""
    with comp.group(f"emit_from_mem{suffix}") as emit_from_mem:
        mem.addr0 = comp.this()["j"]
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

    wrap.control += cb.par(
        cb.if_(
            wrap.get_cell("eq0").out,
            wrap.get_group("i_eq_0"),
            wrap.get_group("emit_from_mem1"),
        ),
        cb.if_(
            wrap.get_cell("eq1").out,
            wrap.get_group("i_eq_1"),
            wrap.get_group("emit_from_mem2"),
        ),
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

    _ = main.cell("together", CompInst("wrap", []))

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
    main.control = cb.invoke(
        main.get_cell("together"), in_i=cb.const(32, 1), in_j=cb.const(32, 3)
    )


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    add_wrap(prog)
    add_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
