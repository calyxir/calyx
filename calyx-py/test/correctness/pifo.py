# pylint: disable=import-error
import fifo
import calyx.builder as cb
import calyx.builder_util as util
import calyx.queue_call as qc


def insert_flow_inference(comp: cb.ComponentBuilder, cmd, flow, boundary, group):
    """The flow is needed when the command is a push.
    If the value to be pushed is less than or equal to {boundary},
    the value belongs to flow 0.
    Otherwise, the value belongs to flow 1.
    This method adds a group to the component {comp} that does this.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, creates a cell {cell} that checks for less-than.
    3. Puts the values {boundary} and {cmd} into the left and right ports of {cell}.
    4. Then puts the answer of the computation into {flow}.
    5. Returns the group that does this.
    """
    cell = comp.lt("flow_inf", 32)
    with comp.group(group) as infer_flow_grp:
        cell.left = boundary
        cell.right = cmd
        flow.write_en = 1
        flow.in_ = cell.out
        infer_flow_grp.done = flow.done
    return infer_flow_grp


def invoke_subqueue(queue_cell, cmd, ans, err) -> cb.invoke:
    """Invokes the cell {queue_cell} with:
    {cmd} passed by value
    {ans} passed by reference
    {err} passed by reference
    """
    return cb.invoke(
        queue_cell,
        in_cmd=cmd,
        ref_ans=ans,
        ref_err=err,
    )


def insert_pifo(prog, name, queue_l, queue_r, boundary):
    """Inserts the component `pifo` into the program.

    The PIFO achieves a 50/50 split between two "flows" or "kinds".
    That is, up to the availability of values, this PIFO seeks to alternate
    between values of the two flows.

    We say "up to availability" because, if one flow is silent and the other
    is active, the active ones gets to emit consecutive values (in temporary
    violation of the 50/50 rule) until the silent flow starts transmitting again.
    At that point we go back to 50/50.

    The PIFO's maximum capacity is 10.
    Let's say the two flows are called `0` and `1`.
    We orchestrate two sub-queues, `queue_l` and `queue_r`, each of capacity 10.
    We maintain a register that points to which of these sub-queues is "hot".
    Start off with `hot` pointing to `queue_l` (arbitrarily).

    - `push(v, PIFO)`:
       + If len(PIFO) = 10, raise an "overflow" err and exit.
       + Otherwise, the charge is to enqueue value `v`.
         Find out which flow `f` the value `v` should go to;
         `f` better be either `0` or `1`.
         Enqueue `v` into `queue_l` if `f` = `0`, and into `queue_r` if `f` = `1`.
         Note that the sub-queue's enqueue method is itself partial: it may raise
         "overflow", in which case we propagate the overflow flag.
    - `pop(PIFO)`:
       + If `len(PIFO)` = 0, raise an "underflow" flag and exit.
       + Try `pop(queue_{hot})`, where we use the value of `hot` to determine
            which sub-queue to pop from:
            `queue_l` if `hot` = 0, and `queue_r` if `hot` = 1.
         * If it succeeds it will return a value `v`; just propagate `v`.
            Also flip `hot` so it points to the other sub-queue.
         * If it fails because of underflow, return `pop(queue_{not-hot})`.
           If the _second_ pop also fails, propagate the error.
           Leave `hot` as it was.
    """

    pifo: cb.ComponentBuilder = prog.component(name)
    cmd = pifo.input("cmd", 32)
    # If this is 0, we pop. If it is 1, we peek. Otherwise, we push the value.

    # Declare the two sub-queues as cells of this component.
    queue_l = pifo.cell("queue_l", queue_l)
    queue_r = pifo.cell("queue_r", queue_r)

    flow = pifo.reg("flow", 1)  # The flow to push to: 0 or 1.
    # We will infer this using a separate component;
    # it is a function of the value being pushed.
    infer_flow = insert_flow_inference(pifo, cmd, flow, boundary, "infer_flow")

    ans = pifo.reg("ans", 32, is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`.

    err = pifo.reg("err", 1, is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.

    len = pifo.reg("len", 32)  # The length of the PIFO.

    # A register that marks the next sub-queue to `pop` from.
    hot = pifo.reg("hot", 1)

    # Some equality checks.
    hot_eq_0 = util.insert_eq(pifo, hot.out, 0, "hot_eq_0", 1)
    hot_eq_1 = util.insert_eq(pifo, hot.out, 1, "hot_eq_1", 1)
    flow_eq_0 = util.insert_eq(pifo, flow.out, 0, "flow_eq_0", 1)
    flow_eq_1 = util.insert_eq(pifo, flow.out, 1, "flow_eq_1", 1)
    len_eq_0 = util.insert_eq(pifo, len.out, 0, "len_eq_0", 32)
    len_eq_10 = util.insert_eq(pifo, len.out, 10, "len_eq_10", 32)
    cmd_eq_0 = util.insert_eq(pifo, cmd, 0, "cmd_eq_0", 32)
    cmd_eq_1 = util.insert_eq(pifo, cmd, 1, "cmd_eq_1", 32)
    cmd_gt_1 = util.insert_gt(pifo, cmd, 1, "cmd_gt_1", 32)
    err_eq_0 = util.insert_eq(pifo, err.out, 0, "err_eq_0", 1)
    err_neq_0 = util.insert_neq(pifo, err.out, cb.const(1, 0), "err_neq_0", 1)

    flip_hot = util.insert_bitwise_flip_reg(pifo, hot, "flip_hot", 1)
    raise_err = util.insert_reg_store(pifo, err, 1, "raise_err")  # set `err` to 1
    lower_err = util.insert_reg_store(pifo, err, 0, "lower_err")  # set `err` to 0
    zero_out_ans = util.insert_reg_store(pifo, ans, 0, "zero_out_ans")

    len_incr = util.insert_incr(pifo, len, "len_incr")  # len++
    len_decr = util.insert_decr(pifo, len, "len_decr")  # len--

    # The main logic.
    pifo.control += [
        cb.par(
            # Was it a pop, peek, or a push? We can do all cases in parallel.
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
                                [  # `hot` is 0. We'll invoke `pop` on `queue_l`.
                                    invoke_subqueue(queue_l, cmd, ans, err),
                                    # Our next step depends on whether `queue_l`
                                    # raised the error flag.
                                    # We can check these cases in parallel.
                                    cb.par(
                                        cb.if_(
                                            err_neq_0[0].out,
                                            err_neq_0[1],
                                            [  # `queue_l` raised an error.
                                                # We'll try to pop from `queue_r`.
                                                # We'll pass it a lowered err
                                                lower_err,
                                                invoke_subqueue(queue_r, cmd, ans, err),
                                            ],
                                        ),
                                        cb.if_(
                                            err_eq_0[0].out,
                                            err_eq_0[1],
                                            [  # `queue_l` succeeded.
                                                # Its answer is our answer.
                                                flip_hot
                                                # We'll just make `hot` point
                                                # to the other sub-queue.
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
                                    invoke_subqueue(queue_r, cmd, ans, err),
                                    cb.par(
                                        cb.if_(
                                            err_neq_0[0].out,
                                            err_neq_0[1],
                                            [
                                                lower_err,
                                                invoke_subqueue(queue_l, cmd, ans, err),
                                            ],
                                        ),
                                        cb.if_(
                                            err_eq_0[0].out,
                                            err_eq_0[1],
                                            [flip_hot],
                                        ),
                                    ),
                                ],
                            ),
                        ),
                        len_decr,  # Decrement the length.
                        # It is possible that an irrecoverable error was raised above,
                        # in which case the length should _not_ in fact be decremented.
                        # However, in that case the PIFO's `err` flag would also
                        # have been raised, and no one will check this length anyway.
                    ],
                ),
            ),
            cb.if_(
                # Did the user call peek?
                cmd_eq_1[0].out,
                cmd_eq_1[1],
                cb.if_(
                    # Yes, the user called peek. But is the queue empty?
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
                                [  # `hot` is 0. We'll invoke `peek` on `queue_l`.
                                    invoke_subqueue(queue_l, cmd, ans, err),
                                    # Our next step depends on whether `queue_l`
                                    # raised the error flag.
                                    cb.if_(
                                        err_neq_0[0].out,
                                        err_neq_0[1],
                                        [  # `queue_l` raised an error.
                                            # We'll try to peek from `queue_r`.
                                            # We'll pass it a lowered `err`.
                                            lower_err,
                                            invoke_subqueue(queue_r, cmd, ans, err),
                                        ],
                                    ),
                                    # Peeking does not affect `hot`.
                                    # Peeking does not affect the length.
                                ],
                            ),
                            # If `hot` is 1, we proceed symmetrically.
                            cb.if_(
                                hot_eq_1[0].out,
                                hot_eq_1[1],
                                [
                                    invoke_subqueue(queue_r, cmd, ans, err),
                                    cb.if_(
                                        err_neq_0[0].out,
                                        err_neq_0[1],
                                        [
                                            lower_err,
                                            invoke_subqueue(queue_l, cmd, ans, err),
                                        ],
                                    ),
                                ],
                            ),
                        ),
                    ],
                ),
            ),
            cb.if_(
                # Did the user call push?
                cmd_gt_1[0].out,
                cmd_gt_1[1],
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
                                # This value should be pushed to queue_l.
                                invoke_subqueue(queue_l, cmd, ans, err),
                            ),
                            cb.if_(
                                flow_eq_1[0].out,
                                flow_eq_1[1],
                                # This value should be pushed to queue_r.
                                invoke_subqueue(queue_r, cmd, ans, err),
                            ),
                        ),
                        len_incr,  # Increment the length.
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
    fifo_l = fifo.insert_fifo(prog, "fifo_l")
    fifo_r = fifo.insert_fifo(prog, "fifo_r")
    pifo = insert_pifo(prog, "pifo", fifo_l, fifo_r, 200)
    qc.insert_main(prog, pifo)
    return prog.program


if __name__ == "__main__":
    build().emit()
