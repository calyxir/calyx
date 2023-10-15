# pylint: disable=import-error
import fifo
import calyx.builder as cb
import calyx.queue_call as qc

MAX_QUEUE_LEN = 10


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
    cell = comp.lt(32)
    with comp.group(group) as infer_flow_grp:
        cell.left = boundary
        cell.right = cmd
        flow.write_en = 1
        flow.in_ = cell.out
        infer_flow_grp.done = flow.done
    return infer_flow_grp


def invoke_subqueue(queue_cell, cmd, value, ans, err) -> cb.invoke:
    """Invokes the cell {queue_cell} with:
    {cmd} passed by value
    {value} passed by value
    {ans} passed by reference
    {err} passed by reference
    """
    return cb.invoke(
        queue_cell,
        in_cmd=cmd,
        in_value=value,
        ref_ans=ans,
        ref_err=err,
    )


def insert_pifo(prog, name, queue_l, queue_r, boundary, stats=None):
    """Inserts the component `pifo` into the program.

    The PIFO achieves a 50/50 split between two "flows" or "kinds".
    That is, up to the availability of values, this PIFO seeks to alternate
    between values of the two flows.

    We say "up to availability" because, if one flow is silent and the other
    is active, the active ones gets to emit consecutive values (in temporary
    violation of the 50/50 rule) until the silent flow starts transmitting again.
    At that point we go back to 50/50.

    The PIFO's maximum capacity is MAX_QUEUE_LEN.
    Let's say the two flows are called `0` and `1`.
    We orchestrate two sub-queues, `queue_l` and `queue_r`,
    each of capacity MAX_QUEUE_LEN.
    We maintain a register that points to which of these sub-queues is "hot".
    Start off with `hot` pointing to `queue_l` (arbitrarily).

    - `push(v, PIFO)`:
       + If len(PIFO) = MAX_QUEUE_LEN, raise an "overflow" err and exit.
       + Otherwise, the charge is to enqueue value `v`.
         * Find out which flow `f` the value `v` should go to;
         `f` better be either `0` or `1`.
         * Enqueue `v` into `queue_l` if `f` = `0`, and into `queue_r` if `f` = `1`.
         * Note that the sub-queue's enqueue method is itself partial: it may raise
         "overflow", in which case we propagate the overflow flag.
         * If the enqueue succeeds, _and_ if a stats component is provided,
         invoke the stats component and tell it that a value of flow `f` was enqueued.
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
    cmd = pifo.input("cmd", 2)
    # If this is 0, we pop. If it is 1, we peek.
    # If it is 2, we push `value` to the queue.
    value = pifo.input("value", 32)  # The value to push to the queue

    # Declare the two sub-queues as cells of this component.
    queue_l = pifo.cell("queue_l", queue_l)
    queue_r = pifo.cell("queue_r", queue_r)

    # If a stats component was provided, declare it as a cell of this component.
    if stats:
        stats = pifo.cell("stats", stats)

    flow = pifo.reg("flow", 1)  # The flow to push to: 0 or 1.
    # We will infer this using a separate component;
    # it is a function of the value being pushed.
    infer_flow = insert_flow_inference(pifo, value, flow, boundary, "infer_flow")

    ans = pifo.reg("ans", 32, is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`.

    err = pifo.reg("err", 1, is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.

    len = pifo.reg("len", 32)  # The length of the PIFO.

    # A register that marks the next sub-queue to `pop` from.
    hot = pifo.reg("hot", 1)

    # Some equality checks.
    hot_eq_0 = pifo.eq_use(hot.out, 0)
    hot_eq_1 = pifo.eq_use(hot.out, 1)
    flow_eq_0 = pifo.eq_use(flow.out, 0)
    flow_eq_1 = pifo.eq_use(flow.out, 1)
    len_eq_0 = pifo.eq_use(len.out, 0)
    len_eq_max_queue_len = pifo.eq_use(len.out, MAX_QUEUE_LEN)
    cmd_eq_0 = pifo.eq_use(cmd, 0)
    cmd_eq_1 = pifo.eq_use(cmd, 1)
    cmd_eq_2 = pifo.eq_use(cmd, 2)
    err_eq_0 = pifo.eq_use(err.out, 0)
    err_neq_0 = pifo.neq_use(err.out, 0)

    flip_hot = pifo.bitwise_flip_reg(hot)
    raise_err = pifo.reg_store(err, 1, "raise_err")  # err := 1
    lower_err = pifo.reg_store(err, 0, "lower_err")  # err := 0
    flash_ans = pifo.reg_store(ans, 0, "flash_ans")  # ans := 0

    len_incr = pifo.incr(len)  # len++
    len_decr = pifo.decr(len)  # len--

    # The main logic.
    pifo.control += [
        cb.par(
            # Was it a pop, peek, or a push? We can do all cases in parallel.
            cb.if_with(
                # Did the user call pop?
                cmd_eq_0,
                cb.if_with(
                    # Yes, the user called pop. But is the queue empty?
                    len_eq_0,
                    [raise_err, flash_ans],  # The queue is empty: underflow.
                    [  # The queue is not empty. Proceed.
                        # We must check if `hot` is 0 or 1.
                        lower_err,
                        cb.par(  # We'll check both cases in parallel.
                            cb.if_with(
                                # Check if `hot` is 0.
                                hot_eq_0,
                                [  # `hot` is 0. We'll invoke `pop` on `queue_l`.
                                    invoke_subqueue(queue_l, cmd, value, ans, err),
                                    # Our next step depends on whether `queue_l`
                                    # raised the error flag.
                                    # We can check these cases in parallel.
                                    cb.par(
                                        cb.if_with(
                                            err_neq_0,
                                            [  # `queue_l` raised an error.
                                                # We'll try to pop from `queue_r`.
                                                # We'll pass it a lowered err
                                                lower_err,
                                                invoke_subqueue(
                                                    queue_r, cmd, value, ans, err
                                                ),
                                            ],
                                        ),
                                        cb.if_with(
                                            err_eq_0,
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
                            cb.if_with(
                                hot_eq_1,
                                [
                                    invoke_subqueue(queue_r, cmd, value, ans, err),
                                    cb.par(
                                        cb.if_with(
                                            err_neq_0,
                                            [
                                                lower_err,
                                                invoke_subqueue(
                                                    queue_l, cmd, value, ans, err
                                                ),
                                            ],
                                        ),
                                        cb.if_with(
                                            err_eq_0,
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
            cb.if_with(
                # Did the user call peek?
                cmd_eq_1,
                cb.if_with(
                    # Yes, the user called peek. But is the queue empty?
                    len_eq_0,
                    [raise_err, flash_ans],  # The queue is empty: underflow.
                    [  # The queue is not empty. Proceed.
                        # We must check if `hot` is 0 or 1.
                        lower_err,
                        cb.par(  # We'll check both cases in parallel.
                            cb.if_with(
                                # Check if `hot` is 0.
                                hot_eq_0,
                                [  # `hot` is 0. We'll invoke `peek` on `queue_l`.
                                    invoke_subqueue(queue_l, cmd, value, ans, err),
                                    # Our next step depends on whether `queue_l`
                                    # raised the error flag.
                                    cb.if_with(
                                        err_neq_0,
                                        [  # `queue_l` raised an error.
                                            # We'll try to peek from `queue_r`.
                                            # We'll pass it a lowered `err`.
                                            lower_err,
                                            invoke_subqueue(
                                                queue_r, cmd, value, ans, err
                                            ),
                                        ],
                                    ),
                                    # Peeking does not affect `hot`.
                                    # Peeking does not affect the length.
                                ],
                            ),
                            # If `hot` is 1, we proceed symmetrically.
                            cb.if_with(
                                hot_eq_1,
                                [
                                    invoke_subqueue(queue_r, cmd, value, ans, err),
                                    cb.if_with(
                                        err_neq_0,
                                        [
                                            lower_err,
                                            invoke_subqueue(
                                                queue_l, cmd, value, ans, err
                                            ),
                                        ],
                                    ),
                                ],
                            ),
                        ),
                    ],
                ),
            ),
            cb.if_with(
                # Did the user call push?
                cmd_eq_2,
                cb.if_with(
                    # Yes, the user called push. But is the queue full?
                    len_eq_max_queue_len,
                    [raise_err, flash_ans],  # The queue is full: overflow.
                    [  # The queue is not full. Proceed.
                        lower_err,
                        # We need to check which flow this value should be pushed to.
                        infer_flow,  # Infer the flow and write it to `fifo_{flow}`.
                        cb.par(
                            cb.if_with(
                                flow_eq_0,
                                # This value should be pushed to queue_l.
                                invoke_subqueue(queue_l, cmd, value, ans, err),
                            ),
                            cb.if_with(
                                flow_eq_1,
                                # This value should be pushed to queue_r.
                                invoke_subqueue(queue_r, cmd, value, ans, err),
                            ),
                        ),
                        cb.if_with(
                            err_eq_0,
                            # If no stats component is provided,
                            # just increment the length.
                            [len_incr]
                            if not stats
                            else [
                                # If a stats component is provided,
                                # Increment the length and also
                                # tell the stats component what flow we pushed.
                                # Do not request a report.
                                len_incr,
                                cb.invoke(
                                    stats,
                                    in_flow=flow.out,
                                    in_report=0,
                                ),
                            ],
                        ),
                    ],
                ),
            ),
        ),
    ]

    return pifo


def insert_stats(prog, name):
    """Inserts a stats component called `name` into the program `prog`.

    It accepts, as input ports, two things:
    - a flag that indicates whether a _report_ is sought.
    - the index of a flow (0 or 1).

    It maintains two output ports, `count_0_out` and `count_1_out`.

    It also maintains two internal registers, `count_0` and `count_1`.

    If the `report` flag is set:
    The component doesn't change the counts, it just drives them to the output ports.

    If the `report` flag is not set:
    The component reads the flow index and increments `count_0` or `count_1` as needed.
    """

    stats: cb.ComponentBuilder = prog.component(name)
    report = stats.input(
        "report", 1
    )  # If 0, increment a counter. If 1, report the counts so far.
    flow = stats.input(
        "flow", 1
    )  # If 0, increment `count_0`. If 1, increment `count_1`.
    stats.output("count_0_out", 32)
    stats.output("count_1_out", 32)

    # Two registers to count the number of times we've been invoked with each flow.
    count_0 = stats.reg("count_0", 32)
    count_1 = stats.reg("count_1", 32)

    # Wiring to increment the appropriate register.
    count_0_incr = stats.incr(count_0)
    count_1_incr = stats.incr(count_1)

    # Equality checks.
    flow_eq_0 = stats.eq_use(flow, 0)
    flow_eq_1 = stats.eq_use(flow, 1)
    report_eq_0 = stats.eq_use(report, 0)

    with stats.continuous:
        stats.this().count_0_out = count_0.out
        stats.this().count_1_out = count_1.out

    # The main logic.
    stats.control += [
        cb.if_with(
            report_eq_0,  # Report is low, so the user wants to update the counts.
            cb.par(
                cb.if_with(flow_eq_0, [count_0_incr]),
                cb.if_with(flow_eq_1, [count_1_incr]),
            ),
        ),
    ]

    return stats


def insert_controller(prog, name, stats):
    """Inserts a controller component called `name` into the program `prog`.

    This component invokes the `stats` component, to which it has a handle,
    to retrieve its latest stats.

    The eventual goal is to have this happen _periodically_.
    For now, we just do it once.
    """

    controller = prog.component(name)
    stats = controller.cell("stats", stats)

    count_0 = controller.reg("count_0", 32)
    count_1 = controller.reg("count_1", 32)

    # The main logic.
    controller.control += [
        # Invoke the stats component. Pass a bogus value for `flow`,
        # but request a report. Store the results in `count_0` and `count_1`.
        cb.invoke(
            stats,
            in_flow=0,
            in_report=1,
            ref_count_0=count_0,
            ref_count_1=count_1,
        ),  # Great, now I have the counts locally.
    ]

    return controller


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    fifo_l = fifo.insert_fifo(prog, "fifo_l")
    fifo_r = fifo.insert_fifo(prog, "fifo_r")
    stats = insert_stats(prog, "stats")
    pifo = insert_pifo(prog, "pifo", fifo_l, fifo_r, 200)
    # controller = insert_controller(prog, "controller", stats)
    qc.insert_main(prog, pifo)
    return prog.program


if __name__ == "__main__":
    build().emit()
