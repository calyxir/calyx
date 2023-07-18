# pylint: disable=import-error
import calyx.builder as cb


def add_eq(comp: cb.ComponentBuilder, port, const, cell, width, group):
    """Adds wiring into component {comp} to check if {port} == {const}.
    1. Within {comp}, creates a group called {group}.
    2. Within {group}, creates a cell called {cell} that checks equality of width {width}.
    3. Puts the values of {port} and {const} into {cell}.
    4. Returns the equality-checking cell and the equality-checking group.
    """
    eq_cell = comp.eq(cell, width)
    with comp.comb_group(group) as eq_group:
        eq_cell.left = port
        eq_cell.right = const
    return eq_cell, eq_group


def add_incr(comp: cb.ComponentBuilder, reg, cell, group):
    """Adds wiring into component {comp} to increment {reg} by 1.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, adds a cell {cell} that computes sums.
    3. Puts the values of {port} and 1 into {cell}.
    4. Then puts the answer of the computation back into {port}.
    4. Returns the add-computing group.
    """
    incr_cell = comp.add(cell, 32)
    with comp.group(group) as incr_group:
        incr_cell.left = reg.out
        incr_cell.right = 1
        reg.write_en = 1
        reg.in_ = incr_cell.out
        incr_group.done = reg.done
    return incr_group


def zero_out_reg(comp: cb.ComponentBuilder, reg, group):
    """Adds wiring into component {comp} to zero out {reg}.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, adds a cell {cell} that computes sums.
    3. Puts the values of {port} and 1 into {cell}.
    4. Then puts the answer of the computation back into {port}.
    4. Returns the add-computing group.
    """
    with comp.group(group) as zero_group:
        reg.in_ = 0
        reg.write_en = 1
        zero_group.done = reg.done
    return zero_group


def mem_load(comp: cb.ComponentBuilder, mem, i, ans, group):
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


def mem_store(comp: cb.ComponentBuilder, mem, i, val, group):
    """Stores a value from one memory into another.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from memory {mem} at address {i}.
    3. Writes the value into memory {ans} at address 0.
    4. Returns the group that does this.
    """
    with comp.group(group) as store_grp:
        mem.addr0 = i
        mem.write_en = 1
        mem.write_data = val
        store_grp.done = mem.done
    return store_grp


def set_flag(comp: cb.ComponentBuilder, flagname, flagval, group):
    """Sets a flag to a value.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, sets the flag {flagname} to {flagval}.
    3. Returns the group that does this.
    """
    with comp.group(group) as flag_grp:
        flagname.in_ = flagval
        flagname.write_en = 1
        flag_grp.done = flagname.done
    return flag_grp


def set_flag_mem(comp: cb.ComponentBuilder, flagname, flagval, group):
    """Sets a flag to a value.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, sets the flag {flagname} to {flagval}.
    3. Returns the group that does this.
    """
    with comp.group(group) as flag_grp:
        flagname.addr0 = 0
        flagname.write_en = cb.const(1, 1)
        flagname.write_data = flagval
        flag_grp.done = flagname.done
    return flag_grp


def add_fifo(prog):
    """Inserts the component `fifo` into the program.

    It has:
    - three inputs, `pop`, `push`, and `payload`.
    - one memory, `mem`, of size 10.
    - four registers, `next_write`, `next_read`, `full`, and `empty`.
    - two ref memories, `ans` and `err`.
    """

    fifo: cb.ComponentBuilder = prog.component("fifo")
    pop = fifo.input("pop", 1)
    push = fifo.input("push", 1)
    payload = fifo.input("payload", 32)

    # A memory cell.
    mem = fifo.mem_d1("mem", 32, 10, 32)

    # Four registers.
    next_write = fifo.reg("next_write", 32)
    next_read = fifo.reg("next_read", 32)
    full = fifo.reg("full", 1)
    empty = fifo.reg("empty", 1)

    # Two ref memories.
    ans = fifo.mem_d1("ans", 32, 1, 32, is_ref=True)
    err = fifo.mem_d1("err", 1, 1, 1, is_ref=True)

    # Additional cells and groups to compute equality and sums
    eq0cell, eq0grp = add_eq(fifo, pop, push, "eq0", 1, "pop_eq_push")
    eq1cell, eq1grp = add_eq(fifo, pop, 1, "eq1", 1, "pop_eq_1")
    eq2cell, eq2grp = add_eq(fifo, push, 1, "eq2", 1, "push_eq_1")
    eq3cell, eq3grp = add_eq(
        fifo, next_read.out, next_write.out, "eq3", 32, "next_read_eq_next_write"
    )

    write_incr = add_incr(fifo, next_write, "add1", "next_write_incr")
    read_incr = add_incr(fifo, next_read, "add2", "next_read_incr")

    eq5acell, eq5agrp = add_eq(fifo, next_write.out, 10, "eq5b", 32, "next_write_eq_10")
    eq5bcell, eq5bgrp = add_eq(fifo, next_read.out, 10, "eq5a", 32, "next_read_eq_10")

    write_wrap = zero_out_reg(fifo, next_write, "next_write_wraparound")
    read_wrap = zero_out_reg(fifo, next_read, "next_read_wraparound")

    # Load and store
    write_to_mem = mem_store(fifo, mem, next_write.out, payload, "write_payload_to_mem")
    read_from_mem = mem_load(fifo, mem, next_read.out, ans, "read_payload_from_mem")

    # Set flags
    raise_full = set_flag(fifo, full, 1, "raise_full_flag")
    lower_full = set_flag(fifo, full, 0, "lower_full_flag")
    raise_empty = set_flag(fifo, empty, 1, "raise_empty_flag")
    lower_empty = set_flag(fifo, empty, 0, "lower_empty_flag")

    eq4acell, eq4agrp = add_eq(fifo, full.out, 1, "eq4a", 1, "is_full")
    eq4bcell, eq4bgrp = add_eq(fifo, empty.out, 1, "eq4b", 1, "is_empty")

    raise_err = set_flag_mem(fifo, err, cb.const(1, 1), "raise_err_flag")
    lower_err = set_flag_mem(fifo, err, cb.const(1, 0), "lower_err_flag")

    fifo.control += [
        cb.if_(
            eq0cell.out,
            eq0grp,
            raise_err,
            cb.par(
                cb.if_(
                    eq1cell.out,
                    eq1grp,
                    cb.if_(
                        eq4bcell.out,
                        eq4bgrp,
                        raise_err,
                        [
                            lower_err,
                            read_from_mem,
                            read_incr,
                            cb.if_(eq5bcell.out, eq5bgrp, read_wrap),
                            cb.if_(eq3cell.out, eq3grp, raise_empty),
                            cb.if_(eq4acell.out, eq4agrp, lower_full),
                        ],
                    ),
                ),
                cb.if_(
                    eq2cell.out,
                    eq2grp,
                    cb.if_(
                        eq4acell.out,
                        eq4agrp,
                        raise_err,
                        [
                            lower_err,
                            write_to_mem,
                            write_incr,
                            cb.if_(eq5acell.out, eq5agrp, write_wrap),
                            cb.if_(eq3cell.out, eq3grp, raise_full),
                            cb.if_(eq4bcell.out, eq4bgrp, lower_empty),
                        ],
                    ),
                ),
            ),
        )
    ]

    return fifo


def add_main(prog, fifo):
    """Inserts the component `main` into the program.
    This will be used to `invoke` the component `fifo`.
    """
    main: cb.ComponentBuilder = prog.component("main")

    ans = main.mem_d1("ans_in", 32, 1, 32, is_external=True)
    err = main.mem_d1("err_in", 1, 1, 1, is_external=True)
    fifo = main.cell("myfifo", fifo)

    ten_pushes = [
        cb.invoke(
            fifo,
            in_pop=cb.const(1, 0),
            in_push=cb.const(1, 1),
            in_payload=cb.const(32, 100 + i),
            ref_ans=ans,
            ref_err=err,
        )
        for i in range(10)
    ]

    pop = cb.invoke(
        fifo,
        in_pop=cb.const(1, 1),
        in_push=cb.const(1, 0),
        ref_ans=ans,
        ref_err=err,
    )

    ten_pops = [pop for _ in range(10)]

    main.control += (
        ten_pushes
        + [
            cb.invoke(
                fifo,
                in_pop=cb.const(1, 0),
                in_push=cb.const(1, 1),
                in_payload=cb.const(32, 110),
                ref_ans=ans,
                ref_err=err,
            ),
            pop,
            cb.invoke(
                fifo,
                in_pop=cb.const(1, 0),
                in_push=cb.const(1, 1),
                in_payload=cb.const(32, 110),
                ref_ans=ans,
                ref_err=err,
            ),
        ]
        + ten_pops
        + [pop]
    )

    # + [push_n(110), pop, push_n(110), ten_pops + [pop]]


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    fifo = add_fifo(prog)
    add_main(prog, fifo)
    return prog.program


if __name__ == "__main__":
    build().emit()
