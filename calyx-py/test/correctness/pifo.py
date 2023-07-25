# pylint: disable=import-error
import fifo
import calyx.builder_util as util
import calyx.builder as cb


def insert_len_update(comp: cb.ComponentBuilder, length, len_0, len_1, group):
    """Updates the length of the PIFO.
    It is just the sum of the lengths of the two FIFOs.
    1. Within component {comp}, creates a group called {group}.
    2. Creates a cell {cell} that computes sums.
    3. Puts the values of {len_0} and {len_1} into {cell}.
    4. Then puts the answer of the computation back into {length}.
    4. Returns the group that does this.
    """
    cell = comp.add("len_adder", 32)
    with comp.group(group) as update_length_grp:
        cell.left = len_0.out
        cell.right = len_1.out
        length.write_en = 1
        length.in_ = cell.out
        update_length_grp.done = length.done
    return update_length_grp


def insert_flow_inference(comp: cb.ComponentBuilder, cmd, flow, group):
    """The flow is needed when the command is a push.
    If the value to be pushed is less than 200, we push to flow 0.
    Otherwise, we push to flow 1.
    This method adds a group to the component {comp} that does this.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, creates a cell {cell} that checks for less-than.
    3. Puts the values of 199 and {cmd} into {cell}.
    4. Then puts the answer of the computation into {flow}.
    4. Returns the group that does this.
    """
    cell = comp.lt("flow_inf", 32)
    with comp.group(group) as infer_flow_grp:
        cell.left = 199
        cell.right = cmd
        flow.write_en = 1
        flow.in_ = cell.out
        infer_flow_grp.done = flow.done
    return infer_flow_grp


def insert_propagate_err(prog, name):
    """A component that propagates an error flag.
    It takes as input an error flag, and sets its own error flag to that value.
    """
    propagate_err: cb.ComponentBuilder = prog.component(name)

    val = propagate_err.input("val", 1)
    err = propagate_err.reg("err", 1, is_ref=True)

    prop_err = util.insert_reg_store(propagate_err, err, val, "prop_err")

    propagate_err.control += [prop_err]

    return propagate_err


def insert_pifo(prog, name):
    """A PIFO that achieves a 50/50 split between two flows.

    Up to the availability of values, this PIFO seeks to alternate 50/50
    between two "flows".

    We say "up to availability" because, if one flow is silent and the other
    is active, the active ones gets to emit consecutive values (in temporary
    violation of the 50/50 rule) until the silent flow starts transmitting again.
    At that point we go back to 50/50.

    Say the PIFO's maximum capacity is 10. Create two FIFOs, each of capacity 10.
    Let's say the two flow are called `0` and `1`, and our FIFOs are called
    `fifo_0` and `fifo_1`.
    Maintain additionally a register that points to which of these FIFOs is "hot".
    Start off with `hot` pointing to `fifo_0` (arbitrarily).
    Maintain `cold` that points to the other fifo.

    - len(PIFO) = len(fifo_0) + len(fifo_1)
    - `push(v, PIFO)`:
       + If len(PIFO) = 10, raise an "overflow" err and exit.
       + Otherwise, the charge is to enqueue value `v`.
         Find out which flow `f` the value `v` should go to;
         `f` better be either `0` or `1`.
         Enqueue `v` into `fifo_f`.
         Note that the FIFO's enqueue method is itself partial: it may raise
         "overflow", in which case we propagate the overflow flag.
    - `pop(PIFO)`:
       + If `len(PIFO)` = 0, raise an "underflow" flag and exit.
       + Try `pop(FIFO_{hot})`.
         * If it succeeds it will return a value `v`; just propagate `v`. Also flip
           `hot` and `cold`.
         * If it fails because of underflow, return `pop(FIFO_{cold})`.
           If the _second_ pop also fails, propagate the error.
           Leave `hot` and `cold` as they were.
    """

    pifo: cb.ComponentBuilder = prog.component(name)

    # Create the two FIFOs and ready them for invocation.
    fifo_0 = pifo.cell("myfifo_0", fifo.insert_fifo(prog, "fifo_0"))
    fifo_1 = pifo.cell("myfifo_1", fifo.insert_fifo(prog, "fifo_1"))

    cmd = pifo.input(
        "cmd", 32
    )  # The command to execute. 0 = pop, nonzero = push that value

    flow = pifo.reg("flow", 1)  # The flow to push to: 0 or 1
    # We will infer this using an external component and the value of `cmd`
    infer_flow = insert_flow_inference(pifo, cmd, flow, "infer_flow")

    ans = pifo.reg("ans", 32, is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`

    err = pifo.reg("err", 1, is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow
    err_0 = pifo.reg("err_fifo_0", 1)  # an error flag dedicated to fifo_1
    err_1 = pifo.reg("err_fifo_1", 1)  # and one for fifo_1
    propagate_err = pifo.cell("prop_err", insert_propagate_err(prog, "propagate_err"))
    # Sometimes we'll need to propagate an error message to the main `err` flag

    len = pifo.reg("len", 32, is_ref=True)  # The length of the PIFO
    len_0 = pifo.reg("len_0", 32)  # The length of fifo_0
    len_1 = pifo.reg("len_1", 32)  # The length of fifo_1

    # Two registers that mark the next FIFO to `pop` from
    hot = pifo.reg("hot", 1)
    cold = pifo.reg("cold", 1)

    # Some equality checks.
    hot_eq_0 = util.insert_eq(pifo, hot.out, cb.const(1, 0), "hot_eq_0", 1)  # hot == 0
    hot_eq_1 = util.insert_eq(pifo, hot.out, 1, "hot_eq_1", 1)  # hot == 1
    flow_eq_0 = util.insert_eq(pifo, flow.out, 0, "flow_eq_0", 1)  # flow == 0
    flow_eq_1 = util.insert_eq(pifo, flow.out, 1, "flow_eq_1", 1)  # flow == 1
    len_eq_0 = util.insert_eq(pifo, len.out, 0, "len_eq_0", 32)  # `len` == 0
    len_eq_10 = util.insert_eq(pifo, len.out, 10, "len_eq_10", 32)  # `len` == 10
    err_0_eq_1 = util.insert_eq(pifo, err_0.out, 1, "err_0_eq_1", 1)  # err_0 == 1
    err_1_eq_1 = util.insert_eq(pifo, err_1.out, 1, "err_1_eq_1", 1)  # err_1 == 1
    cmd_eq_0 = util.insert_eq(pifo, cmd, cb.const(32, 0), "cmd_eq_0", 32)  # cmd == 0
    cmd_neq_0 = util.insert_neq(pifo, cmd, cb.const(32, 0), "cmd_neq_0", 32)  # cmd != 0

    swap = util.reg_swap(pifo, hot, cold, "swap")  # Swap `hot` and `cold`.
    raise_err = util.insert_reg_store(pifo, err, 1, "raise_err")  # set `err` to 1
    zero_out_ans = util.insert_reg_store(pifo, ans, 0, "zero_out_ans")  # zero out `ans`
    update_length = insert_len_update(pifo, len, len_0, len_1, "update_length")

    # The main logic.
    pifo.control += [
        cb.par(
            # Was it a pop or a push? We can do both cases in parallel.
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
                        # We must check if `hot` is 0 or 1.
                        cb.par(  # We'll check both cases in parallel.
                            cb.if_(
                                # Check if `hot` is 0.
                                hot_eq_0[0].out,
                                hot_eq_0[1],
                                [  # `hot` is 0. We'll invoke `pop` on `fifo_0`.
                                    cb.invoke(  # First we call pop
                                        fifo_0,
                                        in_cmd=cb.const(32, 0),
                                        ref_ans=ans,  # Its answer is our answer.
                                        ref_err=err_0,  # We sequester its error.
                                        ref_len=len_0,
                                    ),
                                    # Now we check if `fifo_0` raised an error.
                                    cb.if_(
                                        err_0_eq_1[0].out,
                                        err_0_eq_1[1],
                                        [  # `fifo_0` raised an error.
                                            # We'll try to pop from `fifo_1`.
                                            cb.invoke(
                                                fifo_1,
                                                in_cmd=cb.const(32, 0),
                                                ref_ans=ans,
                                                # Its answer is our answer.
                                                ref_err=err_1,
                                                # We sequester its error.
                                                ref_len=len_1,
                                            ),
                                            cb.if_(
                                                # If `fifo_1` also raised an error,
                                                # we propagate it and zero out `ans`.
                                                err_1_eq_1[0].out,
                                                err_1_eq_1[1],
                                                [  # `fifo_1` raised an error.
                                                    cb.invoke(
                                                        propagate_err,
                                                        in_val=cb.const(1, 1),
                                                        ref_err=err,
                                                    ),
                                                    zero_out_ans,
                                                ],
                                            ),
                                        ],
                                        [  # `fifo_0` did not raise an error.
                                            # Its answer is our answer.
                                            # We'll just swap `hot` and `cold`.
                                            swap,
                                        ],
                                    ),
                                ],
                            ),
                            cb.if_(
                                # Check if `hot` is 1.
                                hot_eq_1[0].out,
                                hot_eq_1[1],
                                [  # `hot` is 1.
                                    # We'll proceed symmetrically.
                                    cb.invoke(
                                        fifo_1,
                                        in_cmd=cb.const(32, 0),
                                        ref_ans=ans,  # Its answer is our answer.
                                        ref_err=err_1,  # We sequester its error.
                                        ref_len=len_1,
                                    ),
                                    # Now we check if `fifo_1` raised an error.
                                    cb.if_(
                                        err_1_eq_1[0].out,
                                        err_1_eq_1[1],
                                        [  # `fifo_1` raised an error.
                                            # We'll try to pop from `fifo_0`.
                                            cb.invoke(
                                                fifo_0,
                                                in_cmd=cb.const(32, 0),
                                                ref_ans=ans,
                                                # Its answer is our answer.
                                                ref_err=err_0,
                                                # We sequester its error.
                                                ref_len=len_0,
                                            ),
                                            # If `fifo_0` also raised an error,
                                            # we propagate it and zero out `ans`.
                                            cb.if_(
                                                err_0_eq_1[0].out,
                                                err_0_eq_1[1],
                                                [  # `fifo_0` raised an error.
                                                    cb.invoke(
                                                        propagate_err,
                                                        in_val=cb.const(1, 1),
                                                        ref_err=err,
                                                    ),
                                                    zero_out_ans,
                                                ],
                                            ),
                                        ],
                                        [  # `fifo_1` did not raise an error.
                                            # Its answer is our answer.
                                            # We'll just swap `hot` and `cold`.
                                            swap,
                                        ],
                                    ),
                                ],
                            ),
                        ),
                        update_length,  # Update the length of the PIFO.
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
                        # We need to check which flow this value should be pushed to.
                        infer_flow,  # Infer the flow and write it to `flow`.
                        cb.par(
                            cb.if_(
                                flow_eq_0[0].out,
                                flow_eq_0[1],
                                # This value should be pushed to flow 0.
                                cb.invoke(
                                    fifo_0,
                                    in_cmd=cmd,
                                    ref_err=err,  # Its error is our error.
                                    ref_len=len_0,
                                    ref_ans=ans,  # Its answer is our answer.
                                ),
                            ),
                            cb.if_(
                                flow_eq_1[0].out,
                                flow_eq_1[1],
                                # This value should be pushed to flow 1.
                                cb.invoke(
                                    fifo_1,
                                    in_cmd=cmd,
                                    ref_err=err,  # Its error is our error.
                                    ref_len=len_1,
                                    ref_ans=ans,  # Its answer is our answer.
                                ),
                            ),
                        ),
                        update_length,  # Update the length of the PIFO.
                    ],
                ),
            ),
        ),
    ]

    return pifo


def insert_main(prog):
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
    commands = main.seq_mem_d1("commands", 32, 15, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, 10, 32, is_external=True)

    # We will use the `invoke` method to call the `pifo` component.
    pifo = main.cell("mypifo", insert_pifo(prog, "pifo"))
    # The pifo component takes three `ref` inputs:
    err = main.reg("err", 1)  # A flag to indicate an error
    ans = main.reg("ans", 32)  # A memory to hold the answer of a pop
    len = main.reg("len", 32)  # A register to hold the len of the queue

    # We will set up a while loop that runs over the command list, relaying
    # the commands to the `pifo` component.
    # It will run until the `err` flag is raised by the `pifo` component.

    # It is handy to have this component, which can additionally raise the `err`
    # flag in case i = 15.
    raise_err_if_i_eq_15 = main.cell(
        "raise_err_if_i_eq_15", fifo.insert_raise_err_if_i_eq_15(prog)
    )

    i = main.reg("i", 32)  # The index of the command we're currently processing
    j = main.reg("j", 32)  # The index on the answer-list we'll write to
    cmd = main.reg("command", 32)  # The command we're currently processing

    incr_i = util.insert_incr(main, i, "incr_i")  # i++
    incr_j = util.insert_incr(main, j, "incr_j")  # j++
    err_eq_zero = util.insert_eq(main, err.out, 0, "err_eq_0", 1)  # is `err` flag down?
    read_cmd = util.mem_read_seqd1(main, commands, i.out, "read_cmd_phase1")
    write_cmd_to_reg = util.mem_write_seqd1_to_reg(
        main, commands, cmd, "read_cmd_phase2"
    )

    cmd_eq_0 = util.insert_eq(main, cmd.out, 0, "cmd_eq_0", 32)
    cmd_eq_1 = util.insert_eq(main, cmd.out, 1, "cmd_eq_1", 32)
    write_ans = util.mem_store_seq_d1(main, ans_mem, j.out, ans.out, "write_ans")

    main.control += [
        cb.while_(
            err_eq_zero[0].out,
            err_eq_zero[1],  # Run while the `err` flag is down
            [
                read_cmd,  # Read `cmd[i]`
                write_cmd_to_reg,  # And write it to `cmd`
                cb.par(  # Process the command
                    cb.if_(
                        # Is this a pop?
                        cmd_eq_0[0].out,
                        cmd_eq_0[1],
                        [  # A pop
                            cb.invoke(  # First we call pop
                                pifo,
                                in_cmd=cmd.out,
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
                    cb.if_(
                        # Is this a push?
                        cmd_eq_1[0].out,
                        cmd_eq_1[1],
                        [
                            # A push
                            cb.invoke(
                                pifo,
                                in_cmd=cmd.out,
                                ref_ans=ans,
                                ref_err=err,
                                ref_len=len,
                            ),
                        ],
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
