# pylint: disable=import-error
import builder_util as util
import calyx.builder as cb


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
    raise_err = util.reg_store(raise_err_if_i_eq_15, err, 1, "raise_err")

    raise_err_if_i_eq_15.control += [
        cb.if_(
            i_eq_15[0].out,
            i_eq_15[1],
            raise_err,
        )
    ]

    return raise_err_if_i_eq_15


def insert_fifo(prog):
    """Inserts the component `fifo` into the program.

    It has:
    - three inputs, `pop`, `push`, and `payload`.
    - one memory, `mem`, of size 10.
    - two registers, `next_write` and `next_read`.
    - three ref registers, `ans`, `err`, and `len`.
    """

    fifo: cb.ComponentBuilder = prog.component("fifo")
    pop = fifo.input("pop", 1)
    push = fifo.input("push", 1)
    payload = fifo.input("payload", 32)

    mem = fifo.mem_d1("mem", 32, 10, 32)

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
    pop_eq_push = util.insert_eq(fifo, pop, push, "pop_eq_push", 1)  # `pop` == `push`
    pop_eq_1 = util.insert_eq(fifo, pop, 1, "pop_eq_1", 1)  # `pop` == 1
    push_eq_1 = util.insert_eq(fifo, push, 1, "push_eq_1", 1)  # `push` == 1
    write_eq_10 = util.insert_eq(
        fifo, write.out, 10, "write_eq_10", 32
    )  # `write` == 10
    read_eq_10 = util.insert_eq(fifo, read.out, 10, "read_eq_10", 32)  # `read` == 10
    len_eq_0 = util.insert_eq(fifo, len.out, 0, "len_eq_0", 32)  # `len` == 0
    len_eq_10 = util.insert_eq(fifo, len.out, 10, "len_eq_10", 32)  # `len` == 10

    # Cells and groups to increment read and write registers
    write_incr = util.insert_incr(fifo, write, "add1", "write_incr")  # write++
    read_incr = util.insert_incr(fifo, read, "add2", "read_incr")  # read++
    len_incr = util.insert_incr(fifo, len, "add5", "len_incr")  # len++
    len_decr = util.insert_decr(fifo, len, "add6", "len_decr")  # len--

    # Cells and groups to modify flags, which are registers
    write_wrap = util.reg_store(fifo, write, 0, "write_wraparound")  # zero out `write`
    read_wrap = util.reg_store(fifo, read, 0, "read_wraparound")  # zero out `read`
    raise_err = util.reg_store(fifo, err, 1, "raise_err")  # set `err` to 1
    zero_out_ans = util.reg_store(fifo, ans, 0, "zero_out_ans")  # zero out `ans`

    # Load and store into an arbitary slot in memory
    write_to_mem = util.mem_store(fifo, mem, write.out, payload, "write_payload_to_mem")
    read_from_mem = util.mem_load(fifo, mem, read.out, ans, "read_payload_from_mem")

    fifo.control += [
        cb.if_(
            pop_eq_push[0].out,
            pop_eq_push[1],
            # Checking if the user called pop and push at the same time,
            # or issued no command.
            [
                raise_err,  # If so, we're done.
                zero_out_ans,  # We zero out the answer register.
            ],
            cb.par(  # If not, we continue.
                cb.if_(
                    # Did the user call pop?
                    pop_eq_1[0].out,
                    pop_eq_1[1],
                    cb.if_(
                        # Yes, the user called pop. But is the queue empty?
                        len_eq_0[0].out,
                        len_eq_0[1],
                        [raise_err, zero_out_ans],  # The queue is empty: underflow.
                        [  # The queue is not empty. Proceed.
                            read_from_mem,  # Read from the queue.
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
                    push_eq_1[0].out,
                    push_eq_1[1],
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
        )
    ]

    return fifo


def insert_main(prog, fifo, raise_err_if_i_eq_15):
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
    # The fifo component takes two `ref` inputs:
    err = main.reg("err", 1)  # A flag to indicate an error
    ans = main.reg("ans", 32)  # A memory to hold the answer of a pop
    len = main.reg("len", 32)  # A register to hold the len of the queue

    # We will set up a while loop that runs over the command list, relaying
    # the commands to the `fifo` component.
    # It will run until the `err` flag is raised by the `fifo` component.
    raise_err_if_i_eq_15 = main.cell("raise_err_if_i_eq_15", raise_err_if_i_eq_15)

    i = main.reg("i", 32)  # The index of the command we're currently processing
    j = main.reg("j", 32)  # The index on the answer-list we'll write to
    command = main.reg("command", 32)  # The command we're currently processing

    incr_i = util.insert_incr(main, i, "add3", "incr_i")  # i = i + 1
    incr_j = util.insert_incr(main, j, "add4", "incr_j")  # j = j + 1
    err_eq_zero = util.insert_eq(main, err.out, 0, "err_eq_0", 1)  # is `err` flag down?
    read_command = util.mem_load(main, commands, i.out, command, "read_command")
    command_eq_zero = util.insert_eq(main, command.out, 0, "command_eq_zero", 32)
    write_ans = util.mem_store(main, ans_mem, j.out, ans.out, "write_ans")

    main.control += [
        cb.while_(
            err_eq_zero[0].out,
            err_eq_zero[1],  # Run while the `err` flag is down
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
                            ref_len=len,
                        ),
                        # AM: if err flag comes back raised,
                        # do not perform this write or this incr
                        write_ans,
                        incr_j,
                    ],
                    cb.invoke(  # A push
                        fifo,
                        in_pop=cb.const(1, 0),
                        in_push=cb.const(1, 1),
                        in_payload=command.out,
                        ref_ans=ans,
                        ref_err=err,
                        ref_len=len,
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
    fifo = insert_fifo(prog)
    raise_err_if_i_eq_15 = insert_raise_err_if_i_eq_15(prog)
    insert_main(prog, fifo, raise_err_if_i_eq_15)
    return prog.program


if __name__ == "__main__":
    build().emit()
