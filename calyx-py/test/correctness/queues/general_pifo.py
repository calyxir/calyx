# pylint: disable=import-error
import fifo
import calyx.builder as cb
import calyx.queue_call as qc

# This determines the maximum possible length of the queue:
# The max length of the queue will be 2^QUEUE_LEN_FACTOR.
QUEUE_LEN_FACTOR = 4


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


def insert_pifo(
    prog,
    name,
    fifos,
    boundary,
    n_flows, # the number of flows
    queue_len_factor=QUEUE_LEN_FACTOR,
    stats=None,
    static=False,
):
    """Inserts the component `pifo` into the program.

    The PIFO achieves a 50/50 split between two "flows" or "kinds".
    That is, up to the availability of values, this PIFO seeks to alternate
    between values of the two flows.

    We say "up to availability" because, if one flow is silent and the other
    is active, the active ones gets to emit consecutive values (in temporary
    violation of the 50/50 rule) until the silent flow starts transmitting again.
    At that point we go back to 50/50.

    The PIFO's maximum capacity is determined by `queue_len_factor`:
        max_queue_len = 2**queue_len_factor
    Let's say the two flows are called `0` and `1`.
    We orchestrate two sub-queues, `queue_l` and `queue_r`,
    each having the same maximum capacity as the PIFO.
    We maintain a register that points to which of these sub-queues is "hot".
    Start off with `hot` pointing to `queue_l` (arbitrarily).

    - `push(v, PIFO)`:
       + If len(PIFO) = `max_queue_len`, raise an "overflow" err and exit.
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
    cmd = pifo.input("cmd", 2) # the size in bits is 2
    # If this is 0, we pop. If it is 1, we peek.
    # If it is 2, we push `value` to the queue.
    value = pifo.input("value", 32)  # The value to push to the queue

    fifo_cells = []
    for num in range(len(fifos)):
        name = "queue_" + str(num)
        cell = pifo.cell(name, fifos[num])
        fifo_cells.append(cell)

    # If a stats component was provided, declare it as a cell of this component.
    if stats:
        stats = pifo.cell("stats", stats, is_ref=True)

    flow = pifo.reg(32, "flow")  # The flow to push to: 0 to n.
    # We will infer this using a separate component;
    # it is a function of the value being pushed.
    
    ans = pifo.reg(32, "ans", is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`.

    err = pifo.reg(1, "err", is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.

    length = pifo.reg(32, "length")  # The active length of the PIFO.

    # A register that marks the next sub-queue to `pop` from.
    hot = pifo.reg(32, "hot")

    max_queue_len = 2**queue_len_factor

    """The flow is needed when the command is a push.
    Takes in boundary, which in a 2-flow queue, presumably divided the total
    workload evenly in two. So we do the math to evenly split the total
    workload in n equally-sized flows (stored in variable divide).
    While value is greater than the current value of the divider, we update the
    divider += divide until the guard is false, which means we've found our flow.
    At that point, for every time we've updated the divider, we have also run 
    the group infer_flow_grp that increments i each time. The final answer, i, 
    ends up in {flow}.
    """
    adder = pifo.add(32, "adder_reg")
    i = pifo.reg(32, "i")

    # (boundary * 2) / n_flows will evenly divide "it" into n equal pieces
    divider = pifo.reg(32, "divider") # divide + (n*divide), where n is the number of times we've looped
    divide = (boundary * 2) // n_flows
    bound_val = pifo.reg(32, "bound_val")
    store_bound_val = pifo.reg_store(bound_val, cb.const(32, divide)) # will always store the boundary value
    
    i_lt_n = pifo.lt_use(divider.out, value, "i_lt_n")
    with pifo.group("infer_flow_grp") as infer_flow_grp:
        adder.left = i.out # checking if the value is < the smallest boundary (divide), if so we 
        #automatically know that the packet belongs in the first flow
        adder.right = divider.out
        flow.write_en = cb.HI
        flow.in_ = adder.out
        infer_flow_grp.done = flow.done

    upd_divider, _ = pifo.add_store_in_reg(divider.out, bound_val.out, divider, "upd_divider")

    # Some equality checks.
    hot_eq_n = pifo.eq_use(hot.out, n_flows-1, cellname="hot_eq_n") #bc 0-based indexing
    len_eq_0 = pifo.eq_use(length.out, 0, cellname="len_eq_0")
    len_eq_max_queue_len = pifo.eq_use(length.out, max_queue_len, cellname="len_eq_maxq")
    cmd_eq_0 = pifo.eq_use(cmd, 0, cellname="cmd_eq_0")
    cmd_eq_1 = pifo.eq_use(cmd, 1, cellname="cmd_eq_1")
    cmd_eq_2 = pifo.eq_use(cmd, 2, cellname="cmd_eq_2")
    err_eq_0 = pifo.eq_use(err.out, 0, cellname="err_eq_0")
    err_neq_0 = pifo.neq_use(err.out, 0, cellname="err_neq_0")

    flip_hot = pifo.incr(hot) # TODO want to make it increment until it
    # reaches n_flows, then loop back around to 0, or could check hot using mod %
    raise_err = pifo.reg_store(err, 1, "raise_err")  # err := 1
    lower_err = pifo.reg_store(err, 0, "lower_err")  # err := 0
    reset_hot = pifo.reg_store(hot, 0, "reset_hot") # hot := 0

    len_incr = pifo.incr(length)  # len++
    len_decr = pifo.decr(length)  # len--

    #n_int = 0
    #loop_lt_n = pifo.lt_use(n.out, cb.const(32, n_flows), "loop_lt_n")
    # incr_n = pifo.incr(n)

    handles = []
    for n in range(n_flows):
        handle = cb.if_with(pifo.eq_use(hot.out, cb.const(32, n)), # const(n, 32)
        invoke_subqueue(fifo_cells[n], cmd, value, ans, err))
        handles.append(handle)

    hot_flow = pifo.reg(32)
    upd_hot_flow, _ = pifo.div_store_in_reg(flow.out, bound_val.out, hot_flow)
    decr_hot_flow = pifo.decr(hot_flow) # have to subtract the index by 1 bc the way we calculate it, it can't be 0 bc you can't divide by 0
    flow_handles = []
    for m in range(n_flows): 
        handle1 = cb.if_with(pifo.eq_use(hot_flow.out, cb.const(32, m)),
        invoke_subqueue(fifo_cells[m], cmd, value, ans, err))
        flow_handles.append(handle1)

    # The main logic.
    pifo.control += [store_bound_val, cb.par(
        # Was it a pop, peek, or a push? We can do all cases in parallel.
        cb.if_with(
            # Did the user call pop?
            cmd_eq_0,
            cb.if_with(
                # Yes, the user called pop. But is the queue empty?
                len_eq_0,
                raise_err,  # The queue is empty: underflow.
                [  # The queue is not empty. Proceed.
                    lower_err,
                    [
                        handles,
                        # Our next step depends on whether `fifos[hot]`
                        # raised the error flag.
                        # We can check these cases in parallel.
                        cb.while_with(
                            err_neq_0,
                            [  # `queue[hot]` raised an error.
                                # We'll try to pop from `queue[hot+1]`.
                                # We'll pass it a lowered err
                                lower_err,
                                cb.if_with(hot_eq_n, flip_hot, reset_hot), # increment hot and invoke_subqueue on the next one
                                handles,
                            ],
                            # `queue[hot+n]` succeeded.
                            # Its answer is our answer.
                        ),
                        cb.if_with(hot_eq_n, flip_hot, reset_hot),
                    ],
                    len_decr,  # Decrement the active length.
                    # It is possible that an irrecoverable error was raised above,
                    # in which case the active length should _not_ in fact be decremented.
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
                raise_err,  # The queue is empty: underflow.
                [  # The queue is not empty. Proceed.
                    # We must check if `hot` is 0 or 1.
                    lower_err,
                    [
                        handles,
                        cb.if_with(
                            err_neq_0,
                            [  # `fifos[hot]` raised an error.
                                # We'll try to peek from `fifos[hot+1]`.
                                # We'll pass it a lowered `err`.
                                lower_err,
                                cb.if_with(hot_eq_n, flip_hot, reset_hot), # increment hot and invoke_subqueue on the next one
                                handles
                            ],
                        ),
                        # Peeking does not affect `hot`.
                        # Peeking does not affect the length.
                    ],                  
                ],
            ),
        ), 
        cb.if_with(
            # Did the user call push?
            cmd_eq_2,
            cb.if_with(
                # Yes, the user called push. But is the queue full?
                len_eq_max_queue_len,
                raise_err,  # The queue is full: overflow.
                [  # The queue is not full. Proceed.
                    lower_err,
                    # We need to check which flow this value should be pushed to.
                    cb.while_with(i_lt_n, [upd_divider, infer_flow_grp]),
                    upd_hot_flow,  # compute which flow is 'hot', ie the flow that should be pushed to based on the value.
                    decr_hot_flow,
                    flow_handles,
                    cb.if_with(
                        err_eq_0,
                        # If no stats component is provided,
                        # just increment the active length.
                        (
                            len_incr
                            if not stats
                            else cb.par(
                                # If a stats component is provided,
                                # Increment the active length and also
                                # tell the stats component what flow we pushed.
                                len_incr,
                                (
                                    cb.static_invoke(stats, in_flow=flow.out)
                                    if static
                                    else cb.invoke(stats, in_flow=flow.out)
                                ),
                            )
                        ),
                    ),
                ],
            ),
        ),
    ) ]

    return pifo

def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    n_flows = 2
    sub_fifos = []
    for n in range(n_flows):
        name = "fifo" + str(n)
        sub_fifo = fifo.insert_fifo(prog, name, QUEUE_LEN_FACTOR)
        sub_fifos.append(sub_fifo)

    pifo = insert_pifo(prog, "pifo", sub_fifos, 200, n_flows)
    qc.insert_main(prog, pifo, 8)
    return prog.program


if __name__ == "__main__":
    build().emit()
