# pylint: disable=import-error
"""Common code factored out, to be imported by the different flow implementations."""
import os
import sys
import inspect

currentdir = os.path.dirname(os.path.abspath(inspect.getfile(inspect.currentframe())))
parentdir = os.path.dirname(currentdir)
sys.path.insert(0, parentdir)

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


def insert_rr_pifo(
    prog,
    name,
    fifos,
    boundaries,
    numflows,  # the number of flows
    queue_len_factor=QUEUE_LEN_FACTOR,
    stats=None,
    static=False,
):
    """Inserts the component `pifo` into the program."""

    pifo: cb.ComponentBuilder = prog.component(name)
    cmd = pifo.input("cmd", 2)  # the size in bits is 2
    # If this is 0, we pop. If it is 1, we peek.
    # If it is 2, we push `value` to the queue.
    value = pifo.input("value", 32)  # The value to push to the queue

    fifo_cells = [pifo.cell(f"queue_{i}", fifo_i) for i, fifo_i in enumerate(fifos)]

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
    og_hot = pifo.reg(32, "og_hot")
    copy_hot = pifo.reg_store(og_hot, hot.out)  # og_hot := hot.out
    restore_hot = pifo.reg_store(hot, og_hot.out)  # hot := og_hot.out

    max_queue_len = 2**queue_len_factor

    # Some equality checks.
    len_eq_0 = pifo.eq_use(length.out, 0)
    len_eq_max_queue_len = pifo.eq_use(length.out, max_queue_len)
    err_eq_0 = pifo.eq_use(err.out, 0)
    err_neq_0 = pifo.neq_use(err.out, 0)

    raise_err = pifo.reg_store(err, 1, "raise_err")  # err := 1
    lower_err = pifo.reg_store(err, 0, "lower_err")  # err := 0

    len_incr = pifo.incr(length)  # len++
    len_decr = pifo.decr(length)  # len--

    # We first create a list of invoke-statement handles.
    # Each invoke is guarded by an equality check on the hot register,
    # and each guard is unique to the subqueue it is associated with.
    # This means we can eventually execute all of these invokes in parallel.
    invoke_subqueues_hot_guard_seq = [
        cb.if_with(
            pifo.eq_use(hot.out, n),
            invoke_subqueue(fifo_cells[n], cmd, value, ans, err),
        )
        for n in range(numflows)
    ]
    invoke_subqueues_hot_guard = cb.par(
        invoke_subqueues_hot_guard_seq
    )  # Execute in parallel.

    # We create a list of invoke-statement handles.
    # Each invoke is guarded by a pair of inequality checks on the value register,
    # and each pair of guards is unique to the subqueue it is associated with.
    # This means we can eventually execute all of these invokes in parallel.
    invoke_subqueues_value_guard_seq = [
        cb.if_with(
            pifo.le_use(value, boundaries[b + 1]),
            cb.if_with(
                pifo.gt_use(value, boundaries[b]),
                invoke_subqueue(fifo_cells[b], cmd, value, ans, err),
            ),
        )
        for b in range(numflows)
    ]
    # invoke_subqueues_value_guard_seq = [
    #     cb.if_with(
    #         pifo.le_use(value, boundaries[b + 1]),
    #         cb.if_with(
    #             pifo.gt_use(value, boundaries[b]),
    #             invoke_subqueue(fifo_cells[b], cmd, value, ans, err),
    #         ),
    #     )
    #     for b in range(numflows)
    # ]

    invoke_zero_edge_case = [
        cb.if_with( #edge case of pushing the value 0
            pifo.eq_use(value, 0), 
            cb.if_with(
                pifo.eq_use(cmd, 2),
                invoke_subqueue(fifo_cells[0], cmd, value, ans, err)),
            )
    ]

    invoke_subqueues_value_guard = cb.par(
        invoke_subqueues_value_guard_seq
    )  # Execute in parallel.

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
            lower_err,
            [
                invoke_subqueues_hot_guard,
                # Our next step depends on whether `fifos[hot]` raised the error flag.
                cb.while_with(
                    err_neq_0,
                    [  # `fifo_cells[hot]` raised an error.
                        # We'll try to pop from `fifo_cells[hot+1]`.
                        # We'll pass it a lowered err
                        lower_err,
                        incr_hot_wraparound,
                        invoke_subqueues_hot_guard,
                    ],  # `queue[hot+n]` succeeded. Its answer is our answer.
                ),
            ],
            incr_hot_wraparound,
            len_decr,
        ],
    )

    peek_logic = cb.if_with(
        len_eq_0,
        raise_err,  # The queue is empty: underflow.
        [  # The queue is not empty. Proceed.
            lower_err,
            copy_hot,
            [
                invoke_subqueues_hot_guard,
                cb.while_with(
                    err_neq_0,
                    [  # `fifo_cells[hot]` raised an error.
                        # We'll try to peek from `fifo_cells[hot+1]`.
                        # We'll pass it a lowered `err`.
                        lower_err,
                        incr_hot_wraparound,
                        # increment hot and invoke_subqueue on the next one
                        invoke_subqueues_hot_guard,
                    ],
                ),
                # Peeking does not affect `hot`.
                # Peeking does not affect the length.
            ],
            restore_hot,
        ],
    )

    push_logic = cb.if_with(
        len_eq_max_queue_len,
        raise_err,  # The queue is full: overflow.
        [  # The queue is not full. Proceed.
            lower_err,
            # We need to check which flow this value should be pushed to.
            invoke_subqueues_value_guard,
            invoke_zero_edge_case,
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
    )

    # Was it a pop, peek, push, or an invalid command?
    # We can do those four cases in parallel.
    pifo.control += pifo.case(
        cmd,
        {
            0: pop_logic,
            1: peek_logic,
            2: push_logic,
            3: raise_err,
        },
    )

    return pifo