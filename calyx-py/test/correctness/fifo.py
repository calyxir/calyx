# pylint: disable=import-error
import calyx.builder as cb


def add_eq(comp: cb.ComponentBuilder, a, b, cell, width):
    """Adds wiring into component {comp} to check if {a} == {b}.
    1. Within {comp}, creates a combinational group called {cell}_group.
    2. Within the group, creates a cell {cell} that checks equalities of width {width}.
    3. Puts the values {a} and {b} into {cell}.
    4. Returns the equality-checking cell and the equality-checking group.
    """
    eq_cell = comp.eq(cell, width)
    with comp.comb_group(f"{cell}_group") as eq_group:
        eq_cell.left = a
        eq_cell.right = b
    return eq_cell, eq_group


def add_eq_mem(comp: cb.ComponentBuilder, mem, i, b, cell, width):
    """Adds wiring into component {comp} to check if {mem[i]} == {b}.
    1. Within {comp}, creates a non-combinational group called {cell}_group.
    2. Within the group, creates a cell {cell} that checks equalities of width {width}.
    3. Puts the values {mem[i]} and {b} into {cell}.
    4. Returns the equality-checking cell and the equality-checking group.
    """
    eq_cell = comp.eq(cell, width)
    with comp.group(f"{cell}_group") as eq_group:
        eq_cell.left = mem.read_data
        eq_cell.right = b
        mem.addr0 = i
        eq_group.done = mem.done
    return eq_cell, eq_group


def add_eq_mem_comb(comp: cb.ComponentBuilder, mem, i, b, cell, width):
    """Adds wiring into component {comp} to check if {mem[i]} == {b}.
    1. Within {comp}, creates a combinational group called {cell}_group.
    2. Within the group, creates a cell {cell} that checks equalities of width {width}.
    3. Puts the values {mem[i]} and {b} into {cell}.
    4. Returns the equality-checking cell and the equality-checking group.
    """
    eq_cell = comp.eq(cell, width)
    with comp.comb_group(f"{cell}_group") as eq_group:
        eq_cell.left = mem.read_data
        eq_cell.right = b
        mem.addr0 = i
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
        ans.addr0 = cb.const(32, 0)
        ans.write_data = mem.read_data
        load_grp.done = ans.done
    return load_grp


def mem_load_to_reg(comp: cb.ComponentBuilder, mem, i, reg, group):
    """Loads a value from one memory into a register.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from memory {mem} at address {i}.
    3. Writes the value into register {reg}.
    4. Returns the group that does this.
    """
    with comp.group(group) as load_grp:
        mem.addr0 = i
        reg.write_en = 1
        reg.in_ = mem.read_data
        load_grp.done = reg.done
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


def mem_store_from_mem(comp: cb.ComponentBuilder, mem, i, val_mem, val_i, group):
    """Stores a value from one memory into another.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from {val_mem} at address {val_i}.
    3. Writes the value into memory {mem} at address i.
    4. Returns the group that does this.
    """
    with comp.group(group) as store_grp:
        mem.addr0 = i
        mem.write_en = 1
        mem.write_data = val_mem.read_data
        val_mem.addr0 = val_i
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
    raise_err = mem_store(fifo, err, 0, cb.const(1, 1), "raise_err")  # set `err` to 1

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

    # The user-facing interface is:
    # - a list of commands (the input)
    #    where each command is a 32-bit unsigned integer, with the following format:
    #    `0`: pop
    #    any other value: push that value
    # - a list of answers (the output).
    commands = main.mem_d1("commands", 32, 15, 32, is_external=True)
    ans_mem = main.mem_d1("ans_mem", 32, 10, 32, is_external=True)

    # We will use the `invoke` method to call the `fifo` component.
    fifo = main.cell("myfifo", fifo)
    # The component takes as `ref` input:
    err = main.mem_d1("err", 1, 1, 1)  # A flag to indicate an error
    ans = main.mem_d1("ans", 32, 1, 32)  # A memory to hold the answer of a pop
    # We will set up a while loop that runs over the command list, relaying
    # the commands to the `fifo` component.
    # It will run until the `err` flag is raised by the `fifo` component or
    # until it reaches the end of the command list.

    i = main.reg("i", 32)  # The index of the command we're currently processing
    j = main.reg("j", 32)  # The index on the answer-list we'll write to
    command = main.reg("command", 32)  # The command we're currently processing

    zero_i = set_flag_reg(main, i, 0, "zero_i")  # zero out `i`
    zero_j = set_flag_reg(main, j, 0, "zero_j")  # zero out `j`
    incr_i = add_incr(main, i, "add3", "incr_i")  # i = i + 1
    incr_j = add_incr(main, j, "add4", "incr_j")  # j = j + 1
    err_eq_zero_comb = add_eq_mem_comb(
        main, err, 0, 0, "err_eq_0_comb", 1
    )  # is the `err` flag down?
    err_eq_zero = add_eq_mem(main, err, 0, 0, "err_eq_0", 1)  # is the `err` flag down?
    # i_eq_15 = add_eq(main, i.out, 15, "i_eq_15", 32)  # is `i` == 15?
    # raise_err = mem_store(main, err, 0, cb.const(1, 1), "raise_err")  # set `err` to 1
    read_command = mem_load_to_reg(main, commands, i.out, command, "read_command")
    command_eq_zero = add_eq(main, command.out, 0, "command_eq_zero", 32)
    write_ans = mem_store_from_mem(main, ans_mem, j.out, ans, 0, "write_ans")

    main.control += [
        zero_i,
        zero_j,
        cb.while_(
            err_eq_zero_comb[0].out,
            err_eq_zero_comb[1],  # Run while the `err` flag is down
            [
                read_command,  # Read the command at `i`
                cb.if_(
                    # Is this a pop or a push?
                    command_eq_zero[0].out,
                    command_eq_zero[1],
                    [  # A pop
                        cb.invoke(  # First we call pop
                            fifo,
                            in_pop=cb.const(1, 1),
                            in_push=cb.const(1, 0),
                            ref_ans=ans,
                            ref_err=err,
                        ),
                        # But was it successful?
                        # We need to check on the `err` flag.
                        # To avoid clashing with the combinational group
                        # `err_eq_zero_comb`, we use a new non-combinational
                        # group `err_eq_zero`.
                        err_eq_zero[1],
                        cb.if_(
                            err_eq_zero[0].out,
                            None,
                            [  # Yes, it was successful: write the answer to ans_mem
                                write_ans,
                                incr_j,
                            ],
                        ),
                    ],
                    cb.invoke(  # A push
                        fifo,
                        in_pop=cb.const(1, 0),
                        in_push=cb.const(1, 1),
                        in_payload=command.out,
                        ref_ans=ans,
                        ref_err=err,
                    ),
                ),
                incr_i,  # Increment the command index
                # cb.if_(
                #     # Did we reach the end of the command list?
                #     i_eq_15[0].out,
                #     i_eq_15[1],
                #     raise_err,  # Yes, raise the error flag.
                # ),
            ],
        ),
    ]


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    fifo = add_fifo(prog)
    add_main(prog, fifo)
    return prog.program


if __name__ == "__main__":
    build().emit()
