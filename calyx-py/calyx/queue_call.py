# pylint: disable=import-error
from calyx import queue_util
import calyx.builder as cb


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

    commands = main.seq_mem_d1("commands", 2, queue_util.MAX_CMDS, 32, is_external=True)
    values = main.seq_mem_d1("values", 32, queue_util.MAX_CMDS, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, queue_util.MAX_CMDS, 32, is_external=True)

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
    lower_err = main.reg_store(err, 0, "lower_err")  # err := 1

    cmd_le_1 = main.le_use(cmd.out, 1)  # cmd <= 1

    read_cmd = main.mem_read_seq_d1(commands, i.out, "read_cmd_phase1")
    write_cmd_to_reg = main.mem_write_seq_d1_to_reg(commands, cmd, "write_cmd_phase2")

    read_value = main.mem_read_seq_d1(values, i.out, "read_value")
    write_value_to_reg = main.mem_write_seq_d1_to_reg(
        values, value, "write_value_to_reg"
    )
    write_ans = main.mem_store_seq_d1(ans_mem, j.out, ans.out, "write_ans")

    i_lt_max_cmds = main.lt_use(i.out, queue_util.MAX_CMDS)
    not_err = main.not_use(err.out)
    
    main.control += [
        cb.while_with(
            i_lt_max_cmds,  # Run while i < MAX_CMDS
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
                cb.if_with(
                    not_err,
                    [
                        cb.if_with(
                            cmd_le_1,  # If the command was a pop or peek,
                            [
                                write_ans,  # Write the answer to the answer list
                                incr_j,  # And increment the answer index.
                            ],
                        ),
                    ],
                ),
                lower_err,  # Lower the error flag
                incr_i,  # Increment the command index
            ],
        ),
    ]
