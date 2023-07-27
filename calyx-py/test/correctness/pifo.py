# pylint: disable=import-error
import fifo
import calyx.builder_util as util
import calyx.builder as cb
import calyx.queue_call as qc


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
    cmd = pifo.input("cmd", 32)  # If this is 0, we pop. Otherwise, we push the value.

    # Create the two FIFOs and ready them for invocation.
    fifo_0 = pifo.cell("myfifo_0", fifo.insert_fifo(prog, "fifo_0"))
    fifo_1 = pifo.cell("myfifo_1", fifo.insert_fifo(prog, "fifo_1"))

    flow = pifo.reg("flow", 1)  # The flow to push to: 0 or 1
    # We will infer this using a separate component and the item to be pushed.
    infer_flow = insert_flow_inference(pifo, cmd, flow, "infer_flow")

    ans = pifo.reg("ans", 32, is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`

    err = pifo.reg("err", 1, is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow

    len = pifo.reg("len", 32)  # The length of the PIFO

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
    cmd_eq_0 = util.insert_eq(pifo, cmd, cb.const(32, 0), "cmd_eq_0", 32)  # cmd == 0
    cmd_neq_0 = util.insert_neq(pifo, cmd, cb.const(32, 0), "cmd_neq_0", 32)  # cmd != 0
    err_eq_0 = util.insert_eq(pifo, err.out, 0, "err_eq_0", 1)  # err == 0
    err_neq_0 = util.insert_neq(
        pifo, err.out, cb.const(1, 0), "err_neq_0", 1
    )  # err != 0

    swap = util.reg_swap(pifo, hot, cold, "swap")  # Swap `hot` and `cold`.
    raise_err = util.insert_reg_store(pifo, err, 1, "raise_err")  # set `err` to 1
    lower_err = util.insert_reg_store(pifo, err, 0, "lower_err")  # set `err` to 0
    zero_out_ans = util.insert_reg_store(pifo, ans, 0, "zero_out_ans")  # zero out `ans`

    incr_len = util.insert_incr(pifo, len, "incr_len")  # len++
    decr_len = util.insert_decr(pifo, len, "decr_len")  # len--

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
                        lower_err,
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
                                        ref_err=err,
                                    ),
                                    # Our next step depends on whether `fifo_0`
                                    # raised the error flag.
                                    # We can check these cases in parallel.
                                    cb.par(
                                        cb.if_(
                                            err_neq_0[0].out,
                                            err_neq_0[1],
                                            [  # `fifo_0` raised an error.
                                                # We'll try to pop from `fifo_1`.
                                                cb.invoke(
                                                    fifo_1,
                                                    in_cmd=cb.const(32, 0),
                                                    ref_ans=ans,
                                                    # Its answer is our answer.
                                                    ref_err=err,
                                                    # Its error is our error,
                                                    # whether it raised one or not.
                                                ),
                                            ],
                                        ),
                                        cb.if_(
                                            err_eq_0[0].out,
                                            err_eq_0[1],
                                            [  # `fifo_0` succeeded.
                                                # Its answer is our answer.
                                                swap
                                                # We'll just swap `hot` and `cold`.
                                            ],
                                        ),
                                    ),
                                ],
                            ),
                            # If `hot` is 1, we proceed symmetrically.
                            cb.if_(
                                hot_eq_1[0].out,
                                hot_eq_1[1],
                                [
                                    cb.invoke(
                                        fifo_1,
                                        in_cmd=cb.const(32, 0),
                                        ref_ans=ans,
                                        ref_err=err,
                                    ),
                                    cb.par(
                                        cb.if_(
                                            err_neq_0[0].out,
                                            err_neq_0[1],
                                            [
                                                cb.invoke(
                                                    fifo_0,
                                                    in_cmd=cb.const(32, 0),
                                                    ref_ans=ans,
                                                    ref_err=err,
                                                ),
                                            ],
                                        ),
                                        cb.if_(
                                            err_eq_0[0].out,
                                            err_eq_0[1],
                                            [swap],
                                        ),
                                    ),
                                ],
                            ),
                        ),
                        decr_len,  # Decrement the length.
                        # It is possible that an irrecoverable error was raised above,
                        # in which case the length should _not_ in fact be decremented.
                        # However, in that case the PIFO's `err` flag would also
                        # have been raised, and no one will check this length anyway.
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
                        lower_err,
                        # We need to check which flow this value should be pushed to.
                        infer_flow,  # Infer the flow and write it to `fifo_{flow}`.
                        cb.par(
                            cb.if_(
                                flow_eq_0[0].out,
                                flow_eq_0[1],
                                # This value should be pushed to flow 0.
                                cb.invoke(  # AM: this does not terminate
                                    fifo_0,
                                    in_cmd=cmd,
                                    ref_ans=ans,  # Its answer is our answer.
                                    ref_err=err,  # Its error is our error.
                                ),
                                # [zero_out_ans] # AM: if you'd like to see it
                                # terminate, just uncomment this line,
                                # which is just a placeholder,
                                # and comment out the `invoke` lines above.
                                # Do the same for the other `cb.invoke` below.
                            ),
                            cb.if_(
                                flow_eq_1[0].out,
                                flow_eq_1[1],
                                # This value should be pushed to flow 1.
                                cb.invoke(  # AM: this does not terminate
                                    fifo_1,
                                    in_cmd=cmd,
                                    ref_ans=ans,  # Its answer is our answer.
                                    ref_err=err,  # Its error is our error.
                                ),
                                # [zero_out_ans] # AM: if you'd like to see it
                                # terminate, just uncomment this line
                            ),
                        ),
                        # AM: incredibly, the line below is one of the sources of
                        # non-termination!! Comment it out as well, if you want
                        # to see the program terminate.
                        incr_len,  # Increment the length.
                        # It is possible that an irrecoverable error was raised above,
                        # in which case the length should _not_ in fact be incremented.
                        # However, in that case the PIFO's `err` flag would also
                        # have been raised, and no one will check this length anyway.
                    ],
                ),
            ),
        ),
    ]

    return pifo


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    pifo = insert_pifo(prog, "pifo")
    qc.insert_main(prog, pifo)
    return prog.program


if __name__ == "__main__":
    build().emit()
