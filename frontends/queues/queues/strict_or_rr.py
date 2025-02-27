# pylint: disable=import-error
import calyx.builder as cb
import calyx.py_ast as ast
from calyx.utils import bits_needed

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


def insert_queue(
    prog,
    name,
    is_round_robin,
    subqueues,
    flow_infer,
    order=None,
    queue_len_factor=QUEUE_LEN_FACTOR,
):
    """
    Inserts the component `pifo` into the program, operating over n flows (where n is `len(subqueues)`).
    If `is_round_robin` is true, it inserts a round robin queue, otherwise it inserts a strict queue.
    `flow_infer` is the component used for flow inference; it must be invoked with an input `value`
    and reference register `flow` of size floor(log_2(n)).
    `order` is used for strict queues to determine the priority of the subqueues.
    `order` must be a permutation of {0, ..., n - 1}.
    """
    numflows = len(subqueues)

    assert is_round_robin or sorted(order) == list(range(numflows))

    pifo: cb.ComponentBuilder = prog.component(name)
    cmd = pifo.input("cmd", 1)  # the size in bits is 1
    # If this is 0, we pop. If it is 1, we push `value` to the queue.
    value = pifo.input("value", 32)  # The value to push to the queue

    subqueue_cells = [
        pifo.cell(f"queue_{i}", queue_i) for i, queue_i in enumerate(subqueues)
    ]

    flow = pifo.reg(bits_needed(numflows - 1), "flow")
    flow_infer = pifo.cell("flow_infer", flow_infer)
    infer_flow = cb.invoke(flow_infer, in_value=value, ref_flow=flow)

    # If the user wants to pop, we will write the popped value to `ans`.
    ans = pifo.reg(32, "ans", is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.
    err = pifo.reg(1, "err", is_ref=True)

    length = pifo.reg(32, "length")  # The active length of the PIFO.
    hot = pifo.reg(32, "hot")  # A register that marks the next sub-queue to `pop` from.
    og_hot = pifo.reg(32, "og_hot")
    copy_hot = pifo.reg_store(og_hot, hot.out)  # og_hot := hot.out
    max_queue_len = 2**queue_len_factor

    # Some equality checks.
    len_eq_0 = pifo.eq_use(length.out, 0)
    len_eq_max_queue_len = pifo.eq_use(length.out, max_queue_len)
    err_is_low = pifo.eq_use(err.out, 0)
    err_is_high = pifo.neq_use(err.out, 0)

    raise_err = pifo.reg_store(err, 1, "raise_err")  # err := 1
    lower_err = pifo.reg_store(err, 0, "lower_err")  # err := 0
    len_incr = pifo.incr(length)  # len++
    len_decr = pifo.decr(length)  # len--

    # We create a dictionary of invokes handles and pass it to the case construct.
    # Each invoke is uniquely guarded by an equality check on the hot register.
    # This means we can execute all of these invokes in parallel and know that
    # only one will succeed.
    invoke_subqueues_hot_guard = pifo.case(
        hot.out,
        {
            n: invoke_subqueue(subqueue_cells[n], cmd, value, ans, err)
            for n in range(numflows)
        },
    )

    # We create a list of invoke-statement handles.
    # Each invoke is uniquely guarded by an equality check on the flow register.
    # This means we can eventually execute all of these invokes in parallel.
    invoke_subqueues_flow_guard = pifo.case(
        flow.out,
        {
            n: (
                invoke_subqueue(subqueue_cells[n], cmd, value, ans, err)
                if is_round_robin
                else invoke_subqueue(
                    subqueue_cells[order.index(n)], cmd, value, ans, err
                )
            )
            for n in range(numflows)
        },
    )

    incr_hot_wraparound = cb.if_with(
        # If hot = numflows - 1, we need to wrap around to 0. Otherwise, we increment.
        pifo.eq_use(hot.out, numflows - 1),
        pifo.reg_store(hot, 0, "reset_hot"),
        pifo.incr(hot),
    )

    pop_logic = cb.if_with(
        len_eq_0,
        raise_err,  # The queue is empty: underflow.
        [  # The queue is not empty. Proceed.
            copy_hot,  # We remember `hot` so we can restore it later.
            raise_err,  # We raise err so we enter the loop body at least once.
            cb.while_with(
                err_is_high,
                [  # We have entered the loop body because `err` is high.
                    # Either we are here for the first time,
                    # or we are here because the previous iteration raised an error
                    # and incremented `hot` for us.
                    # We'll try to pop from `subqueue_cells[hot]`.
                    # We'll pass it a lowered `err`.
                    lower_err,
                    invoke_subqueues_hot_guard,
                    incr_hot_wraparound,  # Increment hot: this will be used
                    # only if the current subqueue raised an error,
                    # and another iteration is needed.
                ],
            ),
            len_decr,
            (
                pifo.reg_store(hot, og_hot.out) if not is_round_robin else ast.Empty
                # If we are not generating a round-robin PIFO,
                # we are generating a strict PIFO.
                # We need to restore `hot` to its original value.
            ),
        ],
    )

    push_logic = cb.if_with(
        len_eq_max_queue_len,
        raise_err,  # The queue is full: overflow.
        [  # The queue is not full. Proceed.
            lower_err,
            # flow := flow of incoming packet
            infer_flow,
            # We'll push to the subqueue that the value belongs to.
            invoke_subqueues_flow_guard,
            # If all went well, we'll increment the length of the queue.
            cb.if_with(err_is_low, len_incr),
        ],
    )

    # Was it a pop or push?
    # We can do those two cases in parallel.
    pifo.control += pifo.case(
        cmd,
        {0: pop_logic, 1: push_logic},
    )

    return pifo
