# pylint: disable=import-error
import calyx.builder as cb


def add_eq(comp: cb.ComponentBuilder, a, b, cell, width):
    """Adds wiring into component {comp} to check if {a} == {b}.
    1. Within {comp}, creates a group called {cell}_group.
    2. Within the group, creates a cell {cell} that checks equalities of width {width}.
    3. Puts the values {a} and {b} into {cell}.
    4. Returns the equality-checking cell and the equality-checking group.
    """
    eq_cell = comp.eq(cell, width)
    with comp.comb_group(f"{cell}_group") as eq_group:
        eq_cell.left = a
        eq_cell.right = b
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
        ans.addr0 = 0
        ans.write_data = mem.read_data
        load_grp.done = ans.done
    return load_grp


def mem_store(comp: cb.ComponentBuilder, mem, i, val, group):
    """Stores a value from one memory into another.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from {val}.
    3. Writes the value into memory {mem} at address i.
    4. Returns the group that does this.
    """
    with comp.group(group) as store_grp:
        mem.addr0 = i
        mem.write_en = 1
        mem.write_data = val
        store_grp.done = mem.done
    return store_grp


def set_flag_reg(comp: cb.ComponentBuilder, flagname, flagval, group):
    """Sets a flag to a value.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, sets the flag {flagname} to {flagval}.
    3. Returns the group that does this.
    Note that it assumes the flag is a register.
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
    Same as the above, but assumes the flag is a memory of size 1.
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

    mem = fifo.mem_d1("mem", 32, 10, 32)

    write = fifo.reg("next_write", 32)  # The next address to write to
    read = fifo.reg("next_read", 32)  # The next address to read from
    full = fifo.reg("full", 1)
    empty = fifo.reg("empty", 1)

    # We will orchestrate `mem`, along with the two pointers above, to
    # simulate a circular queue of size 10.
    # `write` == `read` can mean the queue is empty or full, so we use
    # the `full` and `empty` flags to keep track of this.

    ans = fifo.mem_d1("ans", 32, 1, 32, is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`

    err = fifo.mem_d1("err", 1, 1, 1, is_ref=True)
    # We'll raise this as a general warning flag.
    # Overflow, underflow, if the user calls pop and push at the same time,
    # or if the user issues no command

    # Cells and groups to compute equality
    pop_eq_push = add_eq(fifo, pop, push, "pop_eq_push", 1)  # is `pop` == `push`?
    pop_eq_1 = add_eq(fifo, pop, 1, "pop_eq_1", 1)  # is `pop` == 1?
    push_eq_1 = add_eq(fifo, push, 1, "push_eq_1", 1)  # is `push` == 1?
    read_eq_write = add_eq(
        fifo, read.out, write.out, "read_eq_write", 32
    )  # is `read` == `write`?
    write_eq_10 = add_eq(fifo, write.out, 10, "write_eq_10", 32)  # is `write` == 10?
    read_eq_10 = add_eq(fifo, read.out, 10, "read_eq_10", 32)  # is `read` == 10?
    full_eq_1 = add_eq(fifo, full.out, 1, "full_eq_1", 1)  # is the `full` flag up?
    empty_eq_1 = add_eq(fifo, empty.out, 1, "empty_eq_1", 1)  # is the `empty` flag up?

    # Cells and groups to increment read and write registers
    write_incr = add_incr(fifo, write, "add1", "write_incr")  # write = write + 1
    read_incr = add_incr(fifo, read, "add2", "read_incr")  # read = read + 1

    # Cells and groups to modify flags, which may be registers or memories of size 1
    write_wrap = set_flag_reg(fifo, write, 0, "write_wraparound")  # zero out `write`
    read_wrap = set_flag_reg(fifo, read, 0, "read_wraparound")  # zero out `read`
    raise_full = set_flag_reg(fifo, full, 1, "raise_full")  # set `full` to 1
    lower_full = set_flag_reg(fifo, full, 0, "lower_full")  # set `full` to 0
    raise_empty = set_flag_reg(fifo, empty, 1, "raise_empty")  # set `empty` to 1
    lower_empty = set_flag_reg(fifo, empty, 0, "lower_empty")  # set `empty` to 0
    raise_err = set_flag_mem(fifo, err, cb.const(1, 1), "raise_err")  # set `err` to 1
    lower_err = set_flag_mem(fifo, err, cb.const(1, 0), "lower_err")  # set `err` to 0

    # Load and store into arbitary slot in memory
    write_to_mem = mem_store(fifo, mem, write.out, payload, "write_payload_to_mem")
    read_from_mem = mem_load(fifo, mem, read.out, ans, "read_payload_from_mem")

    fifo.control += [
        cb.if_(
            pop_eq_push[0].out,
            pop_eq_push[1],
            # The user called pop and push at the same time, or issued no command.
            raise_err,
            cb.par(
                cb.if_(
                    # Did the user call pop?
                    pop_eq_1[0].out,
                    pop_eq_1[1],
                    cb.if_(
                        # Yes, the user called pop. But is the queue empty?
                        empty_eq_1[0].out,
                        empty_eq_1[1],
                        raise_err,  # The queue is empty: underflow.
                        [  # The queue is not empty. Proceed.
                            lower_err,  # Clear the error flag.
                            read_from_mem,  # Read from the queue.
                            read_incr,  # Increment the read pointer.
                            cb.if_(
                                # Wrap around if necessary.
                                read_eq_10[0].out,
                                read_eq_10[1],
                                read_wrap,
                            ),
                            cb.if_(
                                # Raise the empty flag if necessary.
                                read_eq_write[0].out,
                                read_eq_write[1],
                                raise_empty,
                            ),
                            cb.if_(
                                # Lower the full flag if necessary.
                                full_eq_1[0].out,
                                full_eq_1[1],
                                lower_full,
                            ),
                        ],
                    ),
                ),
                cb.if_(
                    # Did the user call push?
                    push_eq_1[0].out,
                    push_eq_1[1],
                    cb.if_(
                        # Yes, the user called push. But is the queue full?
                        full_eq_1[0].out,
                        full_eq_1[1],
                        raise_err,  # The queue is full: overflow.
                        [  # The queue is not full. Proceed.
                            lower_err,  # Clear the error flag.
                            write_to_mem,  # Write to the queue.
                            write_incr,  # Increment the write pointer.
                            cb.if_(
                                # Wrap around if necessary.
                                write_eq_10[0].out,
                                write_eq_10[1],
                                write_wrap,
                            ),
                            cb.if_(
                                # Raise the full flag if necessary.
                                read_eq_write[0].out,
                                read_eq_write[1],
                                raise_full,
                            ),
                            cb.if_(
                                # Lower the empty flag if necessary.
                                empty_eq_1[0].out,
                                empty_eq_1[1],
                                lower_empty,
                            ),
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
        ten_pushes  # These will succeed
        + [
            cb.invoke(  # This will fail
                fifo,
                in_pop=cb.const(1, 0),
                in_push=cb.const(1, 1),
                in_payload=cb.const(32, 110),
                ref_ans=ans,
                ref_err=err,
            ),
            pop,  # This will succeed
            cb.invoke(  # As will this
                fifo,
                in_pop=cb.const(1, 0),
                in_push=cb.const(1, 1),
                in_payload=cb.const(32, 110),
                ref_ans=ans,
                ref_err=err,
            ),
        ]
        + ten_pops  # These will succeed
        + [pop]  # This will fail
    )


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    fifo = add_fifo(prog)
    add_main(prog, fifo)
    return prog.program


if __name__ == "__main__":
    build().emit()
