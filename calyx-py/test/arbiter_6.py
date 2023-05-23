# pylint: disable=import-error
import calyx.builder as cb


def add_eq(comp: cb.ComponentBuilder, port, const, cell, group):
    """Adds wiring into component {comp} to check if {port} == {const}.
    1. Within {comp}, creates a group called {group}.
    2. Within {group}, creates a cell called {cell} that checks equality.
    3. Puts the values of {port} and {const} into {cell}.
    4. Returns the equality-checking cell and the equality-checking group.
    """
    eq_cell = comp.eq(cell, 32)
    with comp.comb_group(group) as eq_group:
        eq_cell.left = port
        eq_cell.right = const
    return eq_cell, eq_group


def add_lt(comp: cb.ComponentBuilder, port, const, cell, group):
    """Adds wiring into component {comp} to check if {port} < {const}.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, creates a cell called {cell} that checks for less-than.
    3. Puts the values of {port} and {const} into {cell}.
    4. Returns the less-than-checking cell and the less-than-checking group.
    """
    lt_cell = comp.lt(cell, 32)
    with comp.comb_group(group) as lt_group:
        lt_cell.left = port
        lt_cell.right = const
    return lt_cell, lt_group


def add_sub(comp: cb.ComponentBuilder, port, const, sub_cell, ans_reg, group):
    """Adds wiring into component {comp} to compute {port} - {const}.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, assumes there is a cell {cell} that computes differences.
    3. Puts the values of {port} and {const} into {cell}.
    4. Then puts the answer of the computation into {ans_reg}.
    4. Returns the sub-checking group.
    """
    # Note, this one is a little different than the others.
    # 1. We assume the subtraction cell already exists.
    # 2. We're not returning the cell, because we don't need to.
    # 3. We write the answer into `ans_reg`.

    with comp.group(group) as sub_group:
        sub_cell.left = port
        sub_cell.right = const
        ans_reg.write_en = 1
        ans_reg.in_ = sub_cell.out
        sub_group.done = ans_reg.done
    return sub_group


def add_mem_load(comp: cb.ComponentBuilder, mem, i, ans, group):
    """Loads a value from one memory into another.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from memory {mem} at address {i}.
    3. Writes the value into memory {ans} at address 0.
    4. Returns the group that does this.
    """
    with comp.group(group) as load_grp:
        mem.addr0 = i
        ans.write_en = 1
        ans.write_data = mem.read_data
        load_grp.done = ans.done
    return load_grp


def add_reg_load(comp: cb.ComponentBuilder, port, ans_reg, group):
    """Creates a group called {group}.
    In that group, loads the value of {port} into {ans_reg}.
    Returns the group.
    """
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
    Accordingly, we assume 0 <= i < 2 and 0 <= j < 12.
    """

    wrap: cb.ComponentBuilder = prog.component("wrap2")
    i = wrap.input("i", 32)
    j = wrap.input("j", 32)

    # Six memory cells, plus an answer cell.
    mems = [wrap.mem_d1(f"mem{i}", 32, 4, 32, is_ref=True) for i in range(6)]
    ans = wrap.mem_d1("ans", 32, 1, 32, is_ref=True)

    # We will need j % 4, so we'll store it in a cell.
    j_mod_4 = wrap.reg("j_mod_4", 32)

    # Additional cells and groups to compute equality and lt
    eq0cell, eq0grp = add_eq(wrap, i, 0, "eq0", "i_eq_0")
    eq1cell, eq1grp = add_eq(wrap, i, 1, "eq1", "i_eq_1")
    lt1cell, lt1grp = add_lt(wrap, j, 4, "lt1", "j_lt_4")
    lt2cell, lt2grp = add_lt(wrap, j, 8, "lt2", "j_lt_8")

    # Load `j` unchanged into `j_mod_4`.
    unchanged = add_reg_load(wrap, j, j_mod_4, "j_unchanged")

    # A subtraction cell and wiring to perform j-4 and j-8.
    sub_cell = wrap.sub("sub", 32)
    sub1cell = add_sub(wrap, j, cb.const(32, 4), sub_cell, j_mod_4, "j_minus_4")
    sub2cell = add_sub(wrap, j, cb.const(32, 8), sub_cell, j_mod_4, "j_minus_8")

    load_from_mems = [
        # Add wiring to load the value `j_mod_4` from all of the memory cells.
        # We'll have to invoke the correct one of these groups later on.
        add_mem_load(wrap, mems[i], j_mod_4.out, ans, f"load_from_mem{i}")
        for i in range(6)
    ]

    wrap.control += [
        cb.if_(
            lt1cell.out,
            lt1grp,
            unchanged,
            cb.if_(lt2cell.out, lt2grp, sub1cell, sub2cell),
        ),
        cb.par(
            cb.if_(
                eq0cell.out,
                eq0grp,
                cb.if_(
                    lt1cell.out,
                    lt1grp,
                    load_from_mems[0],
                    cb.if_(
                        lt2cell.out,
                        lt2grp,
                        load_from_mems[1],
                        load_from_mems[2],
                    ),
                ),
            ),
            cb.if_(
                eq1cell.out,
                eq1grp,
                cb.if_(
                    lt1cell.out,
                    lt1grp,
                    load_from_mems[3],
                    cb.if_(
                        lt2cell.out,
                        lt2grp,
                        load_from_mems[4],
                        load_from_mems[5],
                    ),
                ),
            ),
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
    Accordingly, we assume 0 <= i < 2 and 0 <= j < 12.
    """

    wrap: cb.ComponentBuilder = prog.component("wrap3")
    i = wrap.input("i", 32)
    j = wrap.input("j", 32)

    # Six memory cells, plus an answer cell.
    mems = [wrap.mem_d1(f"mem{i}", 32, 4, 32, is_ref=True) for i in range(6)]
    ans = wrap.mem_d1("ans", 32, 1, 32, is_ref=True)

    # We will need j % 4, so we'll store it in a cell.
    j_mod_4 = wrap.reg("j_mod_4", 32)

    # Additional cells to compute equality, and lt
    eq0cell, eq0grp = add_eq(wrap, i, 0, "eq0", "i_eq_0")
    eq1cell, eq1grp = add_eq(wrap, i, 1, "eq1", "i_eq_1")
    eq2cell, eq2grp = add_eq(wrap, i, 2, "eq2", "i_eq_2")
    ltcell, ltgrp = add_lt(wrap, j, 4, "lt", "j_lt_4")

    # Load `j` unchanged into `j_mod_4`.
    unchanged = add_reg_load(wrap, j, j_mod_4, "j_unchanged")

    # A subtraction cell and wiring to perform j-4.
    sub_cell = wrap.sub("sub", 32)
    subcell = add_sub(wrap, j, cb.const(32, 4), sub_cell, j_mod_4, "j_minus_4")

    emit_from_mems = [
        add_mem_load(wrap, mems[i], j_mod_4.out, ans, f"load_from_mem{i}")
        for i in range(6)
    ]

    wrap.control += [
        cb.if_(ltcell.out, ltgrp, unchanged, subcell),
        cb.par(
            cb.if_(
                eq0cell.out,
                eq0grp,
                cb.if_(ltcell.out, ltgrp, emit_from_mems[0], emit_from_mems[1]),
            ),
            cb.if_(
                eq1cell.out,
                eq1grp,
                cb.if_(ltcell.out, ltgrp, emit_from_mems[2], emit_from_mems[3]),
            ),
            cb.if_(
                eq2cell.out,
                eq2grp,
                cb.if_(ltcell.out, ltgrp, emit_from_mems[4], emit_from_mems[5]),
            ),
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

    # Six memory cells, plus an two answer cells.

    [mem_a, mem_b, mem_c, mem_d, mem_e, mem_f] = [
        main.mem_d1(f"mem{i}", 32, 4, 32, is_external=True) for i in range(6)
    ]
    out2 = main.mem_d1("out2", 32, 1, 32, is_external=True)
    out3 = main.mem_d1("out3", 32, 1, 32, is_external=True)

    together2 = main.cell("together2", wrap2)
    together3 = main.cell("together3", wrap3)

    main.control += [
        cb.invoke(
            together2,
            in_i=cb.const(32, 1),
            in_j=cb.const(32, 11),
            ref_mem1=mem_a,
            ref_mem2=mem_b,
            ref_mem3=mem_c,
            ref_mem4=mem_d,
            ref_mem5=mem_e,
            ref_mem6=mem_f,
            ref_ans=out2,
        ),
        cb.invoke(
            together3,
            in_i=cb.const(32, 2),
            in_j=cb.const(32, 7),
            ref_mem1=mem_a,
            ref_mem2=mem_b,
            ref_mem3=mem_c,
            ref_mem4=mem_d,
            ref_mem5=mem_e,
            ref_mem6=mem_f,
            ref_ans=out3,
        ),
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
