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


def insert_main(prog, queue):
    """Inserts the component `main` into the program.
    This will be used to `invoke` the component `queue` and feed it a list of commands.
    """
    main: cb.ComponentBuilder = prog.component("main")

    # The user-facing interface of the `main` component is:
    # - a list of commands (the input)
    #    where each command is a 32-bit unsigned integer, with the following format:
    #    `0`: pop
    #    any other value: push that value
    # - a list of answers (the output).
    #
    # The user-facing interface of the `queue` component is:
    # - one input, `cmd`.
    #    where each command is a 32-bit unsigned integer, with the following format:
    #    `0`: pop
    #    any other value: push that value
    # - one ref register, `ans`, into which the result of a pop is written.
    # - one ref register, `err`, which is raised if an error occurs.

    commands = main.seq_mem_d1("commands", 32, 15, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, 10, 32, is_external=True)

    # The two components we'll use:
    queue = main.cell("myqueue", queue)
    raise_err_if_i_eq_15 = main.cell(
        "raise_err_if_i_eq_15", insert_raise_err_if_i_eq_15(prog)
    )

    # We will use the `invoke` method to call the `queue` component.
    # The queue component takes two inputs by reference and one input directly.
    # The two `ref` inputs:
    err = main.reg("err", 1)  # A flag to indicate an error
    ans = main.reg("ans", 32)  # A memory to hold the answer of a pop

    # We will set up a while loop that runs over the command list, relaying
    # the commands to the `queue` component.
    # It will run until the `err` flag is raised by the `queue` component.

    i = main.reg("i", 32)  # The index of the command we're currently processing
    j = main.reg("j", 32)  # The index on the answer-list we'll write to
    cmd = main.reg("command", 32)  # The command we're currently processing

    zero_i = util.insert_reg_store(main, i, 0, "zero_i")  # zero out `i`
    zero_j = util.insert_reg_store(main, j, 0, "zero_j")  # zero out `j`
    incr_i = util.insert_incr(main, i, "incr_i")  # i++
    incr_j = util.insert_incr(main, j, "incr_j")  # j++
    err_eq_0 = util.insert_eq(main, err.out, 0, "err_eq_0", 1)  # is `err` flag down?
    cmd_eq_0 = util.insert_eq(main, cmd.out, 0, "cmd_eq_0", 32)  # cmd == 0
    cmd_neq_0 = util.insert_neq(
        main, cmd.out, cb.const(32, 0), "cmd_neq_0", 32
    )  # cmd != 0

    read_cmd = util.mem_read_seqd1(main, commands, i.out, "read_cmd_phase1")
    write_cmd_to_reg = util.mem_write_seqd1_to_reg(
        main, commands, cmd, "write_cmd_phase2"
    )

    write_ans = util.mem_store_seq_d1(main, ans_mem, j.out, ans.out, "write_ans")

    main.control += [
        zero_i,
        zero_j,
        cb.while_(
            err_eq_0[0].out,
            err_eq_0[1],  # Run while the `err` flag is down
            [
                read_cmd,  # Read `commands[i]`
                write_cmd_to_reg,  # Write it to `cmd`
                cb.par(  # Now, in parallel, act based on the value of `cmd`
                    cb.if_(
                        # Is this a pop?
                        cmd_eq_0[0].out,
                        cmd_eq_0[1],
                        [  # A pop
                            cb.invoke(  # First we call pop
                                queue,
                                in_cmd=cmd.out,
                                ref_ans=ans,
                                ref_err=err,
                            ),
                            # AM: my goal is that,
                            # if err flag comes back raised,
                            # we do not perform this write or this incr_j
                            write_ans,
                            incr_j,
                        ],
                    ),
                    cb.if_(  # Is this a push?
                        cmd_neq_0[0].out,
                        cmd_neq_0[1],
                        cb.invoke(  # A push
                            queue,
                            in_cmd=cmd.out,
                            ref_ans=ans,
                            ref_err=err,
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
