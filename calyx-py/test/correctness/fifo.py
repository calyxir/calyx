# pylint: disable=import-error
import calyx.builder as cb
import calyx.builder_util as util


def insert_raise_err_if_i_eq_15(prog):
    """Inserts a the component `raise_err_if_i_eq_15` into the program.

    It has:
    - one input, `i`.
    - one ref register, `err`.

    If `i` equals 15, it raises the `err` flag.
    """
    raise_err_if_i_eq_15: cb.ComponentBuilder = prog.component("raise_err_if_i_eq_15")
    i = raise_err_if_i_eq_15.input("i", 32)
    err = raise_err_if_i_eq_15.reg("err", 1, is_ref=True)

    i_eq_15 = util.insert_eq(raise_err_if_i_eq_15, i, 15, "i_eq_15", 32)
    raise_err = util.insert_reg_store(raise_err_if_i_eq_15, err, 1, "raise_err")

    raise_err_if_i_eq_15.control += [
        cb.if_(
            i_eq_15[0].out,
            i_eq_15[1],
            raise_err,
        )
    ]

    return raise_err_if_i_eq_15


def insert_fifo(prog, name):
    """Inserts the component `fifo` into the program.

    It has:
    - one input, `cmd`.
    - one memory, `mem`, of size 10.
    - two registers, `next_write` and `next_read`.
    - three ref registers, `ans`, `err`, and `len`.
    """

    fifo: cb.ComponentBuilder = prog.component(name)
    cmd = fifo.input("cmd", 32)  # If this is 0, we pop. Otherwise, we push the value.

    mem = fifo.seq_mem_d1("mem", 32, 10, 32)

    write = fifo.reg("next_write", 32)  # The next address to write to
    read = fifo.reg("next_read", 32)  # The next address to read from

    # We will orchestrate `mem`, along with the two pointers above, to
    # simulate a circular queue of size 10.

    ans = fifo.reg("ans", 32, is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`

    err = fifo.reg("err", 1, is_ref=True)
    # We'll raise this as a general error flag:
    # overflow,
    # underflow,
    # if the user calls pop and push at the same time,
    # or if the user issues no command.

    len = fifo.reg("len", 32, is_ref=True)  # The length of the queue

    # Cells and groups to compute equality
    cmd_eq_0 = util.insert_eq(fifo, cmd, 0, "cmd_eq_0", 32)  # `cmd` == 0
    cmd_neq_0 = util.insert_neq(
        fifo, cmd, cb.const(32, 0), "cmd_neq_0", 32
    )  # `cmd` != 0
    write_eq_10 = util.insert_eq(
        fifo, write.out, 10, "write_eq_10", 32
    )  # `write` == 10
    read_eq_10 = util.insert_eq(fifo, read.out, 10, "read_eq_10", 32)  # `read` == 10
    len_eq_0 = util.insert_eq(fifo, len.out, 0, "len_eq_0", 32)  # `len` == 0
    len_eq_10 = util.insert_eq(fifo, len.out, 10, "len_eq_10", 32)  # `len` == 10

    # Cells and groups to increment read and write registers
    write_incr = util.insert_incr(fifo, write, "write_incr")  # write++
    read_incr = util.insert_incr(fifo, read, "read_incr")  # read++
    len_incr = util.insert_incr(fifo, len, "len_incr")  # len++
    len_decr = util.insert_decr(fifo, len, "len_decr")  # len--

    # Cells and groups to modify flags, which are registers
    write_wrap = util.insert_reg_store(
        fifo, write, 0, "write_wraparound"
    )  # zero out `write`
    read_wrap = util.insert_reg_store(
        fifo, read, 0, "read_wraparound"
    )  # zero out `read`
    raise_err = util.insert_reg_store(fifo, err, 1, "raise_err")  # set `err` to 1
    zero_out_ans = util.insert_reg_store(fifo, ans, 0, "zero_out_ans")  # zero out `ans`

    # Load and store into an arbitary slot in memory
    write_to_mem = util.mem_store_seq_d1(
        fifo, mem, write.out, cmd, "write_payload_to_mem"
    )
    read_from_mem = util.mem_read_seqd1(
        fifo, mem, read.out, "read_payload_from_mem_phase1"
    )
    write_to_ans = util.mem_write_seqd1_to_reg(
        fifo, mem, ans, "read_payload_from_mem_phase2"
    )

    fifo.control += [
        cb.par(
            cb.if_(
                # Did the user call pop?
                cmd_eq_0[0].out,
                cmd_eq_0[1],
                cb.if_(
                    # Yes, the user called pop. But is the queue empty?
                    len_eq_0[0].out,
                    len_eq_0[1],
                    [raise_err, zero_out_ans],  # The queue is empty: underflow.
                    [  # The queue is not empty. Proceed.
                        read_from_mem,  # Read from the queue.
                        write_to_ans,  # Write the answer to the answer register.
                        read_incr,  # Increment the read pointer.
                        cb.if_(
                            # Wrap around if necessary.
                            read_eq_10[0].out,
                            read_eq_10[1],
                            read_wrap,
                        ),
                        len_decr,  # Decrement the length.
                    ],
                ),
            ),
            cb.if_(
                # Did the user call push?
                cmd_neq_0[0].out,
                cmd_neq_0[1],
                cb.if_(
                    # Yes, the user called push. But is the queue full?
                    len_eq_10[0].out,
                    len_eq_10[1],
                    [raise_err, zero_out_ans],  # The queue is full: overflow.
                    [  # The queue is not full. Proceed.
                        write_to_mem,  # Write to the queue.
                        write_incr,  # Increment the write pointer.
                        cb.if_(
                            # Wrap around if necessary.
                            write_eq_10[0].out,
                            write_eq_10[1],
                            write_wrap,
                        ),
                        len_incr,  # Increment the length.
                    ],
                ),
            ),
        ),
    ]

    return fifo


def insert_main(prog):
    """Inserts the component `main` into the program.
    This will be used to `invoke` the component `fifo`.
    """
    main: cb.ComponentBuilder = prog.component("main")

    # The user-facing interface of the `main` component is:
    # - a list of commands (the input)
    #    where each command is a 32-bit unsigned integer, with the following format:
    #    `0`: pop
    #    any other value: push that value
    # - a list of answers (the output).
    commands = main.seq_mem_d1("commands", 32, 15, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, 10, 32, is_external=True)

    # The two components we'll use:
    fifo = main.cell("myfifo", insert_fifo(prog, "fifo"))
    raise_err_if_i_eq_15 = main.cell(
        "raise_err_if_i_eq_15", insert_raise_err_if_i_eq_15(prog)
    )

    # We will use the `invoke` method to call the `fifo` component.
    # The fifo component takes three `ref` inputs:
    err = main.reg("err", 1)  # A flag to indicate an error
    ans = main.reg("ans", 32)  # A memory to hold the answer of a pop
    len = main.reg("len", 32)  # A register to hold the len of the queue

    # We will set up a while loop that runs over the command list, relaying
    # the commands to the `fifo` component.
    # It will run until the `err` flag is raised by the `fifo` component.

    i = main.reg("i", 32)  # The index of the command we're currently processing
    j = main.reg("j", 32)  # The index on the answer-list we'll write to
    command = main.reg("command", 32)  # The command we're currently processing

    zero_i = util.insert_reg_store(main, i, 0, "zero_i")  # zero out `i`
    zero_j = util.insert_reg_store(main, j, 0, "zero_j")  # zero out `j`
    incr_i = util.insert_incr(main, i, "incr_i")  # i = i + 1
    incr_j = util.insert_incr(main, j, "incr_j")  # j = j + 1
    err_eq_0 = util.insert_eq(main, err.out, 0, "err_eq_0", 1)  # is `err` flag down?
    cmd_eq_0 = util.insert_eq(main, command.out, 0, "cmd_eq_0", 32)  # is `command` 0?
    cmd_neq_0 = util.insert_neq(
        main, command.out, cb.const(32, 0), "cmd_neq_0", 32
    )  # is `command` 0?

    read_command = util.mem_read_seqd1(main, commands, i.out, "read_command_phase1")
    write_command_to_reg = util.mem_write_seqd1_to_reg(
        main, commands, command, "write_command_phase2"
    )

    write_ans = util.mem_store_seq_d1(main, ans_mem, j.out, ans.out, "write_ans")

    main.control += [
        zero_i,
        zero_j,
        cb.while_(
            err_eq_0[0].out,
            err_eq_0[1],  # Run while the `err` flag is down
            [
                read_command,  # Read `commands[i]`
                write_command_to_reg,  # Write it to `command`
                cb.par(
                    cb.if_(
                        # Is this a pop?
                        cmd_eq_0[0].out,
                        cmd_eq_0[1],
                        [  # A pop
                            cb.invoke(  # First we call pop
                                fifo,
                                in_cmd=command.out,
                                ref_ans=ans,
                                ref_err=err,
                                ref_len=len,
                            ),
                            # AM: if err flag comes back raised,
                            # do not perform this write or this incr
                            write_ans,
                            incr_j,
                        ],
                    ),
                    cb.if_(  # Is this a push?
                        cmd_neq_0[0].out,
                        cmd_neq_0[1],
                        cb.invoke(  # A push
                            fifo,
                            in_cmd=command.out,
                            ref_ans=ans,
                            ref_err=err,
                            ref_len=len,
                        ),
                    ),
                ),
                incr_i,  # Increment the command index
                cb.invoke(  # If i = 15, raise error flag
                    raise_err_if_i_eq_15, in_i=i.out, ref_err=err
                ),  # AM: hella hacky
            ],
        ),
    ]


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    insert_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
