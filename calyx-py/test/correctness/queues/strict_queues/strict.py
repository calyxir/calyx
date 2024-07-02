# pylint: disable=import-error
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


def insert_strict_pifo(
    prog,
    name,
    fifos,
    boundaries,
    numflows,
    order,
    queue_len_factor=QUEUE_LEN_FACTOR
):
    """Inserts the component `pifo` into the program."""

    pifo: cb.ComponentBuilder = prog.component(name)
    cmd = pifo.input("cmd", 2)  # the size in bits is 2
    # If this is 0, we pop. If it is 1, we peek.
    # If it is 2, we push `value` to the queue.
    value = pifo.input("value", 32)  # The value to push to the queue

    fifo_cells = [pifo.cell(f"queue_{i}", fifo_i) for i, fifo_i in enumerate(fifos)]

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
    err_is_low = pifo.eq_use(err.out, 0)
    err_is_high = pifo.neq_use(err.out, 0)

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
            pifo.le_use(value, boundaries[b + 1]),  # value <= boundaries[b+1]
            (
                invoke_subqueue(fifo_cells[order.index(b)], cmd, value, ans, err)
                # In the specical case when b = 0,
                # we don't need to check the lower bound and we can just `invoke`.
                if b == 0
                # Otherwise, we need to check the lower bound and `invoke`
                # only if the value is in the interval.
                else cb.if_with(
                    pifo.gt_use(value, boundaries[b]),  # value > boundaries[b]
                    invoke_subqueue(fifo_cells[order.index(b)], cmd, value, ans, err),
                )
            ),
        )
        for b in range(numflows)
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
            copy_hot,
            [
                invoke_subqueues_hot_guard,
                # Our next step depends on whether `fifos[hot]` raised the error flag.
                cb.while_with(
                    err_is_high,
                    [  # `fifo_cells[hot]` raised an error.
                        # We'll try to pop from `fifo_cells[hot+1]`.
                        # We'll pass it a lowered err
                        lower_err,
                        incr_hot_wraparound,
                        invoke_subqueues_hot_guard,
                    ],  # `queue[hot+n]` succeeded. Its answer is our answer.
                ),
            ],
            restore_hot,
            len_decr,
        ],
    )

    peek_logic = cb.if_with(
        len_eq_0,
        raise_err,  # The queue is empty: underflow.
        [  # The queue is not empty. Proceed.
            lower_err,
            copy_hot, # We remember `hot` so we can restore it later.
            [
                invoke_subqueues_hot_guard,
                cb.while_with(
                    err_is_high,
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
            restore_hot, # Peeking must not affect `hot`, so we restore it.
        ],
    )

    push_logic = cb.if_with(
        len_eq_max_queue_len,
        raise_err,  # The queue is full: overflow.
        [  # The queue is not full. Proceed.
            lower_err,
            # We'll push to the subqueue that the value belongs to.
            invoke_subqueues_value_guard,
            # If all went well, we'll increment the length of the queue.
            cb.if_with(err_is_low, len_incr),
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

def build(numflows):
    """Top-level function to build the program."""

    if numflows == 2:
        boundaries = [0, 200, 400]
        order = [1, 0]
    elif numflows == 3:
        boundaries = [0, 133, 266, 400]
        order = [1, 2, 0]
    elif numflows == 4:
        boundaries = [0, 100, 200, 300, 400]
        order = [3, 0, 2, 1]
    elif numflows == 5:
        boundaries = [0, 80, 160, 240, 320, 400]
        order = [0, 1, 2, 3, 4]
    elif numflows == 6:
        boundaries = [0, 66, 100, 200, 220, 300, 400]
        order = [3, 1, 5, 2, 4, 0]
    else:
        raise ValueError("Unsupported number of flows")

    num_cmds = int(sys.argv[1])

    prog = cb.Builder()
    sub_fifos = [
        fifo.insert_fifo(prog, f"fifo{i}", QUEUE_LEN_FACTOR) for i in range(numflows)
    ]
    pifo = insert_strict_pifo(prog, "pifo", sub_fifos, boundaries, numflows, order)
    qc.insert_main(prog, pifo, num_cmds)
    return prog.program
