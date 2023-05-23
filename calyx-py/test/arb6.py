# pylint: disable=import-error
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
        eq_cell.left = port
        eq_cell.right = const
    return eq_cell, eq_group


def add_lt(comp: cb.ComponentBuilder, port, const, cell, group):
    """Adds wiring into component `comp` to check `port < const`.
    1. Within component `comp`, creates a group called `group`.
    2. Within `group`, creates a cell called `cell` that checks for lt.
    3. Puts the values of `port` and `const` into `cell`.
    4. Returns the lt-checking cell and the lt-checking group.
    """
    lt_cell = comp.lt(cell, 32)
    with comp.comb_group(group) as lt_group:
        lt_cell.left = port
        lt_cell.right = const
    return lt_cell, lt_group


def add_sub(comp: cb.ComponentBuilder, port, const, sub_cell, ans_reg, group):
    """Adds wiring into component `comp` to compute `port - const`.
    1. Within component `comp`, creates a group called `group`.
    2. Within `group`, assumes there is a cell `cell` that computes the difference.
    3. Puts the values of `port` and `const` into `cell`.
    4. Then puts the answer of the computation into `ans_reg`
    4. Returns the sub-checking group.
    """
    with comp.group(group) as sub_group:
        sub_cell.left = port
        sub_cell.right = const
        ans_reg.write_en = 1
        ans_reg.in_ = sub_cell.out
        sub_group.done = ans_reg.done
    return sub_group


def add_mem_load(comp: cb.ComponentBuilder, mem, j, ans, group):
    """Loads a value from one memory into another.
    1. Within component `comp`, creates a group called `group`.
    2. Within `group`, reads from memory `mem` at address `j`.
    3. Writes the value into memory `ans` at address 0.
    4. Returns the group that does this.
    """
    with comp.group(group) as load_grp:
        mem.addr0 = j
        ans.write_en = 1
        ans.write_data = mem.read_data
        load_grp.done = ans.done
    return load_grp


def add_reg_load(comp: cb.ComponentBuilder, port, ans_reg, group):
    """Creates a group called `group`.
    In that, loads the value of `port` into `ans_reg`.
    Returns the group."""
    with comp.group(group) as grp:
        ans_reg.write_en = 1
        ans_reg.in_ = port
        grp.done = ans_reg.done
    return grp


def add_wrap2(prog):
    """Inserts the component `wrap2` into the program.

    It has:
    - two inputs, `i` and `j`.
    - six ref memories, `mem1` through `mem6`, of size 4 each.
    - one ref memory, `ans`.

    The invoker wants to pretend that there are actually
    _two_ memories of size _12_ each.
    The invoker wants to index the memories while living in this fiction.

    This component will return mem[i][j], but indexed according to the fiction.
    Accordingly, 0 <= i < 2 and 0 <= j < 12.
    """

    wrap: cb.ComponentBuilder = prog.component("wrap2")
    wrap.input("i", 32)
    wrap.input("j", 32)

    # AM, quality of life:
    # `input` has no return value, so I'm forced to immediately call `this()`
    # to get the ports I just created.
    i = wrap.this()["i"]
    j = wrap.this()["j"]

    # Six memory cells, plus an answer cell.
    mems = [wrap.mem_d1(f"mem{i}", 32, 4, 32, is_ref=True) for i in range(1, 7)]
    ans = wrap.mem_d1("ans", 32, 1, 32, is_ref=True)

    # We will need j % 4, so we'll store it in a cell.
    j_mod_4 = wrap.reg("j_mod_4", 32)

    # Additional cells to compute equality, lt, and difference
    eq0 = add_eq(wrap, i, 0, "eq0", "i_eq_0")
    eq1 = add_eq(wrap, i, 1, "eq1", "i_eq_1")
    lt1 = add_lt(wrap, j, 4, "lt1", "j_lt_4")
    unchanged = add_reg_load(wrap, j, j_mod_4, "j_unchanged")
    lt2 = add_lt(wrap, j, 8, "lt2", "j_lt_8")

    sub_cell = wrap.sub("sub", 32)
    sub1 = add_sub(wrap, j, cb.const(32, 4), sub_cell, j_mod_4, "j_less_4")
    sub2 = add_sub(wrap, j, cb.const(32, 8), sub_cell, j_mod_4, "j_less_8")

    emit_from_mems = [
        add_mem_load(wrap, mems[k - 1], j_mod_4.out, ans, f"load_from_mem{k}")
        for k in range(1, 7)
    ]

    wrap.control += [
        cb.if_(lt1[0].out, lt1[1], unchanged),
        cb.par(
            cb.if_(eq0[0].out, eq0[1], emit_from_mems[0]),
            cb.if_(eq1[0].out, eq1[1], emit_from_mems[1]),
        ),
    ]

    return wrap


def add_wrap3(prog):
    """Inserts the component `wrap2` into the program.

    It has:
    - two inputs, `i` and `j`.
    - six ref memories, `mem1` through `mem6`, of size 4 each.
    - one ref memory, `ans`.

    The invoker wants to pretend that there are actually
    _two_ memories of size _12_ each.
    The invoker wants to index the memories while living in this fiction.

    This component will return mem[i][j], but indexed according to the fiction.
    Accordingly, 0 <= i < 2 and 0 <= j < 12.
    """

    wrap: cb.ComponentBuilder = prog.component("wrap3")
    wrap.input("i", 32)
    wrap.input("j", 32)

    # AM, quality of life:
    # `input` has no return value, so I'm forced to immediately call `this()`
    # to get the ports I just created.
    i = wrap.this()["i"]
    j = wrap.this()["j"]

    # Six memory cells, plus an answer cell.
    mems = [wrap.mem_d1(f"mem{i}", 32, 4, 32, is_ref=True) for i in range(1, 7)]
    ans = wrap.mem_d1("ans", 32, 1, 32, is_ref=True)

    # We will need j % 4, so we'll store it in a cell.
    j_mod_4 = wrap.reg("j_mod_4", 32)

    # Additional cells to compute equality, lt, and difference
    eq0 = add_eq(wrap, i, 0, "eq0", "i_eq_0")
    eq1 = add_eq(wrap, i, 1, "eq1", "i_eq_1")
    eq2 = add_eq(wrap, i, 2, "eq2", "i_eq_2")
    lt1 = add_lt(wrap, j, 4, "lt", "j_lt_4")
    unchanged = add_reg_load(wrap, j, j_mod_4, "j_unchanged")

    sub_cell = wrap.sub("sub", 32)
    sub1 = add_sub(wrap, j, cb.const(32, 4), sub_cell, j_mod_4, "j_less_4")

    emit_from_mems = [
        add_mem_load(wrap, mems[k - 1], j_mod_4.out, ans, f"load_from_mem{k}")
        for k in range(1, 7)
    ]

    wrap.control += [
        cb.if_(lt1[0].out, lt1[1], unchanged),
        cb.par(
            cb.if_(eq0[0].out, eq0[1], emit_from_mems[0]),
            cb.if_(eq1[0].out, eq1[1], emit_from_mems[1]),
        ),
    ]

    return wrap


def add_main(prog, wrap2, wrap3):
    """Inserts the component `main` into the program.
    This will be used to `invoke` the component `wrap`.

    For now, I'd like to pass it memory cells `A` and `B` by reference,
    along with the inputs i = 1, j = 3.
    """
    main: cb.ComponentBuilder = prog.component("main")
    A = main.mem_d1("A", 32, 4, 32, is_external=True)
    B = main.mem_d1("B", 32, 4, 32, is_external=True)
    C = main.mem_d1("C", 32, 4, 32, is_external=True)
    D = main.mem_d1("D", 32, 4, 32, is_external=True)
    E = main.mem_d1("E", 32, 4, 32, is_external=True)
    F = main.mem_d1("F", 32, 4, 32, is_external=True)

    out2 = main.mem_d1("out2", 32, 1, 32, is_external=True)
    out3 = main.mem_d1("out3", 32, 1, 32, is_external=True)

    # AM, quality of life:
    # Would be nice to have a way to do this in a more `builder` way.
    together2 = main.cell("together2", wrap2)
    together3 = main.cell("together3", wrap3)

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
    main.control += [
        cb.invoke(together2, in_i=cb.const(32, 1), in_j=cb.const(32, 11)),
        cb.invoke(together3, in_i=cb.const(32, 2), in_j=cb.const(32, 7)),
    ]


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    wrap2 = add_wrap2(prog)
    wrap3 = add_wrap3(prog)
    add_main(prog, wrap2, wrap3)
    return prog.program


if __name__ == "__main__":
    build().emit()
