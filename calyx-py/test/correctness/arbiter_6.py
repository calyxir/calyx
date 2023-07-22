# pylint: disable=import-error
import calyx.builder_util as util
import calyx.builder as cb


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
    eq0cell, eq0grp = util.insert_eq(wrap, i, 0, "i_eq_0", 32)
    eq1cell, eq1grp = util.insert_eq(wrap, i, 1, "i_eq_1", 32)
    lt1cell, lt1grp = util.insert_lt(wrap, j, 4, "j_lt_4", 32)
    lt2cell, lt2grp = util.insert_lt(wrap, j, 8, "j_lt_8", 32)

    # Load `j` unchanged into `j_mod_4`.
    unchanged = util.insert_reg_store(wrap, j_mod_4, j, "j_unchanged")

    # A subtraction cell and wiring to perform j-4 and j-8.
    sub_cell = wrap.sub("sub", 32)
    sub1cell = util.insert_sub(wrap, j, cb.const(32, 4), sub_cell, j_mod_4, "j_minus_4")
    sub2cell = util.insert_sub(wrap, j, cb.const(32, 8), sub_cell, j_mod_4, "j_minus_8")

    load_from_mems = [
        # Add wiring to load the value `j_mod_4` from all of the memory cells.
        # We'll have to invoke the correct one of these groups later on.
        util.insert_mem_load_to_mem(
            wrap, mems[i], j_mod_4.out, ans, cb.const(32, 0), f"load_from_mem{i}"
        )
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
    eq0cell, eq0grp = util.insert_eq(wrap, i, 0, "i_eq_0", 32)
    eq1cell, eq1grp = util.insert_eq(wrap, i, 1, "i_eq_1", 32)
    eq2cell, eq2grp = util.insert_eq(wrap, i, 2, "i_eq_2", 32)
    ltcell, ltgrp = util.insert_lt(wrap, j, 4, "j_lt_4", 32)

    # Load `j` unchanged into `j_mod_4`.
    unchanged = util.insert_reg_store(wrap, j_mod_4, j, "j_unchanged")

    # A subtraction cell and wiring to perform j-4.
    sub_cell = wrap.sub("sub", 32)
    subcell = util.insert_sub(wrap, j, cb.const(32, 4), sub_cell, j_mod_4, "j_minus_4")

    emit_from_mems = [
        util.insert_mem_load_to_mem(
            wrap, mems[i], j_mod_4.out, ans, cb.const(32, 0), f"load_from_mem{i}"
        )
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
        main.mem_d1(name, 32, 4, 32, is_external=True)
        for name in ["A", "B", "C", "D", "E", "F"]
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
            ref_mem0=mem_a,
            ref_mem1=mem_b,
            ref_mem2=mem_c,
            ref_mem3=mem_d,
            ref_mem4=mem_e,
            ref_mem5=mem_f,
            ref_ans=out2,
        ),
        cb.invoke(
            together3,
            in_i=cb.const(32, 2),
            in_j=cb.const(32, 7),
            ref_mem0=mem_a,
            ref_mem1=mem_b,
            ref_mem2=mem_c,
            ref_mem3=mem_d,
            ref_mem4=mem_e,
            ref_mem5=mem_f,
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
