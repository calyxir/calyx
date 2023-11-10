# pylint: disable=import-error
import calyx.builder as cb

MAX_CMDS = 15


def insert_main(prog, queue):
    """Inserts the component `main` into the program.
    This will be used to `invoke` the component `queue` and feed it a list of commands.
    This component will directly interface with external memories and will
    finally populate an external memory with the answers.
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
    # The user-facing interface of the `queue` component is assumed to be:
    # - input `cmd`
    #    where each command is a 2-bit unsigned integer, with the following format:
    #    `0`: pop
    #    `1`: peek
    #    `2`: push
    # - input `value`
    #   which is a 32-bit unsigned integer. If `cmd` is `2`, push this value.
    # - one ref register, `ans`, into which the result of a pop or peek is written.
    # - one ref register, `err`, which is raised if an error occurs.

    # We set up the external memories.
    commands = main.seq_mem_d1("commands", 2, MAX_CMDS, 32, is_external=True)
    values = main.seq_mem_d1("values", 32, MAX_CMDS, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, 10, 32, is_external=True)

    # We'll invoke the queue component, which takes two inputs by reference
    # and one input directly.
    queue = main.cell("myqueue", queue)
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
    update_i_neq_max_cmds, _ = main.neq_store_in_reg(
        i.out,
        cb.const(32, MAX_CMDS),
        "i_neq_MAX_CMDS",
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
                        update_i_neq_max_cmds,
                        # Did this increment make us need to break?
                    ],
                ),
            ],
        ),
    ]

    return main


def insert_runner(prog, queue, name, stats_component):
    """Inserts the component `name` into the program.
    This will be used to `invoke` the component `queue` and feed it one command.
    This component is designed to be invoked by some other component, and does not
    directly interface with external memories.
    """
    assert (
        name != "main"
    ), "This method is not designed for the creation of `main`-style components."

    runner: cb.ComponentBuilder = prog.component(name)

    # We take a stats component by reference,
    # but all we'll really do with it is pass it to the queue component.
    stats = runner.cell("stats_runner", stats_component, is_ref=True)

    # We'll invoke the queue component.
    queue = runner.cell("myqueue", queue)

    # The user-facing interface of this component is captured by a number
    # of items that are passed to this component by reference.
    #
    # - 1: `commands`, a list of commands.
    #    Where each command is a 2-bit unsigned integer with the following format:
    #    `0`: pop
    #    `1`: peek
    #    `2`: push
    # - 2: `values`, a list of values.
    #    Where each value is a 32-bit unsigned integer.
    #    The value at `i` is pushed if the command at `i` is `2`.
    # - 3: `has_ans`, a 1-bit unsigned integer.
    #    We raise/lower this to indicate whether the queue had a reply to the command.
    # - 4: `component_ans`, a 32-bit unsigned integer.
    #    We put in this register the answer to the command, if any.
    # - 5: `component_err`, a 1-bit unsigned integer.
    #    We raise/lower it to indicates whether an error occurred
    #    and the queue should no longer be invoked.
    #
    # The user-facing interface of the `queue` component is assumed to be:
    # - input `cmd`
    #    where each command is a 2-bit unsigned integer, with the following format:
    #    `0`: pop
    #    `1`: peek
    #    `2`: push
    # - input `value`
    #   which is a 32-bit unsigned integer. If `cmd` is `2`, push this value.
    # - ref register `ans`, into which the result of a pop or peek is written.
    # - ref register `err`, which is raised if an error occurs.

    # Our memories and registers, all of which are passed to us by reference.
    commands = runner.seq_mem_d1("commands", 2, MAX_CMDS, 32, is_ref=True)
    values = runner.seq_mem_d1("values", 32, MAX_CMDS, 32, is_ref=True)
    has_ans = runner.reg("has_ans", 1, is_ref=True)
    ans = runner.reg("component_ans", 32, is_ref=True)
    err = runner.reg("component_err", 1, is_ref=True)

    i = runner.reg("i", 32)  # The index of the command we're currently processing
    cmd = runner.reg("command", 2)  # The command we're currently processing
    value = runner.reg("value", 32)  # The value we're currently processing

    incr_i = runner.incr(i)  # i++
    cmd_le_1 = runner.le_use(cmd.out, 1)  # cmd <= 1, meaning cmd is pop or peek

    # Wiring to perform `cmd := commands[i]` and `value := values[i]`.
    read_cmd = runner.mem_read_seq_d1(commands, i.out, "read_cmd_phase1")
    write_cmd_to_reg = runner.mem_write_seq_d1_to_reg(commands, cmd, "write_cmd_phase2")
    read_value = runner.mem_read_seq_d1(values, i.out, "read_value")
    write_value_to_reg = runner.mem_write_seq_d1_to_reg(
        values, value, "write_value_to_reg"
    )

    # Wiring to raise/lower flags and compute a negation.
    raise_has_ans = runner.reg_store(has_ans, 1, "raise_has_ans")
    lower_has_ans = runner.reg_store(has_ans, 0, "lower_has_ans")
    err_neg = runner.not_use(err.out)

    # Wiring that raises `err` iff `i = MAX_CMDS`.
    check_if_out_of_cmds, _ = runner.eq_store_in_reg(
        i.out, cb.const(32, MAX_CMDS), "i_eq_MAX_CMDS", 32, err
    )

    runner.control += [
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
            ref_stats=stats,
        ),
        # We're back from the invoke, and it's time for some post-mortem analysis.
        cb.if_with(
            err_neg,  # If there was no error
            [
                cb.if_with(
                    cmd_le_1,  # If the command was a pop or peek
                    [raise_has_ans],  # then raise the `has_ans` flag
                    [lower_has_ans],  # else lower the `has_ans` flag
                ),
                incr_i,  # Increment the command index
                check_if_out_of_cmds,  # If we're out of commands, raise `err`
            ],
        ),
    ]

    return runner
