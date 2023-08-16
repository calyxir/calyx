# pylint: disable=import-error
import calyx.builder as cb
import calyx.builder_util as util

MAX_CMDS = 15
ANS_MEM_LEN = 10


def insert_raise_err_if_i_eq_max_cmds(prog):
    """Inserts a the component `raise_err_if_i_eq_MAX_CMDS` into the program.

    It has:
    - one input, `i`.
    - one ref register, `err`.

    If `i` equals MAX_CMDS, it raises the `err` flag.
    """
    raise_err_if_i_eq_max_cmds: cb.ComponentBuilder = prog.component(
        "raise_err_if_i_eq_MAX_CMDS"
    )
    i = raise_err_if_i_eq_max_cmds.input("i", 32)
    err = raise_err_if_i_eq_max_cmds.reg("err", 1, is_ref=True)

    i_eq_max_cmds = raise_err_if_i_eq_max_cmds.eq_use(i, MAX_CMDS, 32)
    raise_err = util.insert_reg_store(raise_err_if_i_eq_max_cmds, err, 1, "raise_err")

    raise_err_if_i_eq_max_cmds.control += [
        cb.if_(
            i_eq_max_cmds[0].out,
            i_eq_max_cmds[1],
            raise_err,
        )
    ]

    return raise_err_if_i_eq_max_cmds


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

    commands = main.seq_mem_d1("commands", 32, MAX_CMDS, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, ANS_MEM_LEN, 32, is_external=True)

    # The two components we'll use:
    queue = main.cell("myqueue", queue)
    raise_err_if_i_eq_max_cmds = main.cell(
        "raise_err_if_i_eq_MAX_CMDS", insert_raise_err_if_i_eq_max_cmds(prog)
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

    incr_i = main.incr(i, 32)  # i++
    incr_j = main.incr(j, 32)  # j++
    err_eq_0 = main.eq_use(err.out, 0, 1)  # is `err` flag down?
    cmd_le_1 = main.le_use(cmd.out, 1, 32)  # cmd <= 1

    read_cmd = util.mem_read_seq_d1(main, commands, i.out, "read_cmd_phase1")
    write_cmd_to_reg = util.mem_write_seq_d1_to_reg(
        main, commands, cmd, "write_cmd_phase2"
    )

    write_ans = util.mem_store_seq_d1(main, ans_mem, j.out, ans.out, "write_ans")

    main.control += [
        cb.while_(
            err_eq_0[0].out,
            err_eq_0[1],  # Run while the `err` flag is down
            [
                read_cmd,  # Read `commands[i]`
                write_cmd_to_reg,  # Write it to `cmd`
                cb.invoke(  # Call the queue with `cmd`
                    queue,
                    in_cmd=cmd.out,
                    ref_ans=ans,
                    ref_err=err,
                ),
                cb.if_(  # If it was a pop or a peek, write ans to the answer list
                    cmd_le_1[0].out,
                    cmd_le_1[1],
                    [  # AM: I'd like to have an additional check hereL
                        # if err flag comes back raised,
                        # we do not perform this write_ans or this incr_j
                        write_ans,
                        incr_j,
                    ],
                ),
                incr_i,  # Increment the command index
                cb.invoke(  # If i = 15, raise error flag
                    raise_err_if_i_eq_max_cmds, in_i=i.out, ref_err=err
                ),  # AM: hella hacky
            ],
        ),
    ]
