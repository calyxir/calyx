# pylint: disable=import-error
import calyx.builder as cb

MAX_CMDS = 15
ANS_MEM_LEN = 10


def insert_main(prog, queue):
    """Inserts the component `main` into the program.
    This will be used to `invoke` the component `queue` and feed it a list of commands.
    """
    main: cb.ComponentBuilder = prog.component("main")

    # The user-facing interface of the `main` component is:
    # - input 1: a list of commands
    #    where each command is a 2-bit unsigned integer, with the following format:
    #    `0`: pop
    #    `1`: peek
    #    `2`: push
    # - input 2: a list of values to push
    #    where each value is a 32-bit unsigned integer
    #    the value at `i` is pushed if the command at `i` is `2`.
    # - output: a list of answers, reflecting any pops or peeks from the queue.
    #
    # The user-facing interface of the `queue` component is:
    # - input `cmd`
    #    where each command is a 2-bit unsigned integer, with the following format:
    #    `0`: pop
    #    `1`: peek
    #    `2`: push
    # - input `value`
    #   which is a 32-bit unsigned integer. If `cmd` is `2`, push this value.
    # - one ref register, `ans`, into which the result of a pop or peek is written.
    # - one ref register, `err`, which is raised if an error occurs.

    commands = main.seq_mem_d1("commands", 2, MAX_CMDS, 32, is_external=True)
    values = main.seq_mem_d1("values", 32, MAX_CMDS, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, 10, 32, is_external=True)

    # The two components we'll use:
    queue = main.cell("myqueue", queue)

    # We will use the `invoke` method to call the `queue` component.
    # The queue component takes two inputs by reference and one input directly.
    # The two `ref` inputs:
    err = main.reg("err", 1)  # A flag to indicate an error
    ans = main.reg("ans", 32)  # A memory to hold the answer of a pop or peek

    # We will set up a while loop that runs over the command list, relaying
    # the commands to the `queue` component.
    # It will run until the `err` flag is raised by the `queue` component.

    i = main.reg("i", 32)  # The index of the command we're currently processing
    j = main.reg("j", 32)  # The index on the answer-list we'll write to
    cmd = main.reg("command", 2)  # The command we're currently processing
    value = main.reg("value", 32)  # The value we're currently processing

    incr_i = main.incr(i)  # i++
    incr_j = main.incr(j)  # j++
    cmd_le_1 = main.le_use(cmd.out, 1)  # cmd <= 1

    read_cmd = main.mem_read_seq_d1(commands, i.out, "read_cmd_phase1")
    write_cmd_to_reg = main.mem_write_seq_d1_to_reg(commands, cmd, "write_cmd_phase2")

    read_value = main.mem_read_seq_d1(values, i.out, "read_value")
    write_value_to_reg = main.mem_write_seq_d1_to_reg(
        values, value, "write_value_to_reg"
    )
    write_ans = main.mem_store_seq_d1(ans_mem, j.out, ans.out, "write_ans")

    loop_goes_on = main.reg(
        "loop_goes_on", 1
    )  # A flag to indicate whether the loop should continue
    update_err_is_down, _ = main.eq_store_in_reg(
        err.out,
        0,
        "err_is_down",
        1,
        loop_goes_on
        # Does the `err` flag say that the loop should continue?
    )
    update_i_neq_15, _ = main.neq_store_in_reg(
        i.out,
        cb.const(32, 15),
        "i_neq_15",
        32,
        loop_goes_on
        # Does the `i` index say that the loop should continue?
    )

    main.control += [
        update_err_is_down,
        cb.while_(
            loop_goes_on.out,  # Run while the `err` flag is down
            [
                read_cmd,
                write_cmd_to_reg,  # `cmd := commands[i]`
                read_value,
                write_value_to_reg,  # `value := values[i]`
                cb.invoke(  # Invoke the queue.
                    queue,
                    in_cmd=cmd.out,
                    in_value=value.out,
                    ref_ans=ans,
                    ref_err=err,
                ),
                update_err_is_down,  # Does `err` say that the loop should be broken?
                cb.if_(
                    loop_goes_on.out,  # If the loop is not meant to be broken...
                    [
                        cb.if_with(
                            cmd_le_1,  # If the command was a pop or peek,
                            [
                                write_ans,  # Write the answer to the answer list
                                incr_j,  # And increment the answer index.
                            ],
                        ),
                        incr_i,  # Increment the command index
                        update_i_neq_15,  # Did this increment make us need to break?
                    ],
                ),
            ],
        ),
    ]
