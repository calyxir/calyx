# pylint: disable=import-error
from calyx.py_ast import CompInst
import calyx.builder as cb


def add_eq(comp: cb.ComponentBuilder, port, const, cell, group):
    """Adds wiring into component `comp` to check `port == const`.
    1. Within component `comp`, creates a group called `group`.
    2. Within `group`, creates a cell called `cell` that checks equality.
    3. Puts the values of `port` and `const` into `cell`.
    4. Returns the equality-checking cell and the equality-checking group.
    """
    eq_cell = comp.eq(cell, 32)
    with comp.comb_group(group) as eq_group:
        eq_cell.left = comp.this()[port]
        eq_cell.right = const
    return eq_cell, eq_group


def add_mem_load(comp: cb.ComponentBuilder, mem, j, ans, group):
    """Loads a value from one memory into another.
    1. Within component `comp`, creates a group called `group`.
    2. Within `group`, reads from memory `mem` at address `j`.
    3. Writes the value into memory `ans` at address 0.
    4. Returns the group that does this.
    """
    with comp.group(group) as load_grp:
        mem.addr0 = comp.this()[j]
        ans.write_en = 1
        ans.write_data = mem.read_data
        load_grp.done = ans.done
    return load_grp


def add_wrap(prog):
    """Inserts the component `wrap` into the program.

    It has:
    - two inputs, `i` and `j`
    - two ref memories, `mem1` and `mem2`
    - one output, `out`

    Assume 0 <= i < 2 and 0 <= j < 4.
    if i == 0, then out = mem1[j]
    if i == 1, then out = mem2[j]
    """

    wrap: cb.ComponentBuilder = prog.component("wrap")
    wrap.input("i", 32)
    wrap.input("j", 32)

    mem1 = wrap.mem_d1("mem1", 32, 4, 32, is_ref=True)
    mem2 = wrap.mem_d1("mem2", 32, 4, 32, is_ref=True)
    ans = wrap.mem_d1("ans", 32, 1, 32, is_ref=True)

    eq0_cell, eq0_grp = add_eq(wrap, "i", 0, "eq0", "i_eq_0")
    eq1_cell, eq1_grp = add_eq(wrap, "i", 1, "eq1", "i_eq_1")
    emit_from_mem1 = add_mem_load(wrap, mem1, "j", ans, "load_from_mem1")
    emit_from_mem2 = add_mem_load(wrap, mem2, "j", ans, "load_from_mem2")

    wrap.control += cb.par(
        cb.if_(eq0_cell.out, eq0_grp, emit_from_mem1),
        cb.if_(eq1_cell.out, eq1_grp, emit_from_mem2),
    )


def add_main(prog):
    """Inserts the component `main` into the program.
    This will be used to `invoke` the component `wrap`.

    For now, I'd like to pass it memory cells `A` and `B` by reference,
    along with the inputs i = 1, j = 3.
    """
    main: cb.ComponentBuilder = prog.component("main")
    _ = main.mem_d1("A", 32, 4, 32, is_external=True)
    _ = main.mem_d1("B", 32, 4, 32, is_external=True)
    _ = main.mem_d1("out", 32, 1, 32, is_external=True)

    # AM, quality of life:
    # Would be nice to have a way to do this in a more `builder` way.
    together = main.cell("together", CompInst("wrap", []))

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
    main.control = cb.invoke(together, in_i=cb.const(32, 1), in_j=cb.const(32, 3))


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    add_wrap(prog)
    add_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
