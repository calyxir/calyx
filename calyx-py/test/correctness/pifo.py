# pylint: disable=import-error
import fifo
import builder_util as util
import calyx.builder as cb


def reg_swap(comp: cb.ComponentBuilder, a, b, group):
    """Swaps the values of two registers.
    1. Within component {comp}, creates a group called {group}.
    2. Reads the value of {a} into a temporary register.
    3. Writes the value of {b} into {a}.
    4. Writes the value of the temporary register into {b}.
    5. Returns the group that does this.
    """
    with comp.group(group) as swap_grp:
        tmp = comp.register("tmp", 1)
        tmp.write_en = 1
        tmp.in_ = a.out
        a.write_en = 1
        a.in_ = b.out
        b.write_en = 1
        b.in_ = tmp.out
        swap_grp.done = b.done
    return swap_grp


def insert_len_update(comp: cb.ComponentBuilder, len, len_fifo_1, len_fifo_2, group):
    """Updates the length of the PIFO.
    It is just the sum of the lengths of the two FIFOs.
    1. Within component {comp}, creates a group called {group}.
    2. Creates a cell {cell} that computes sums.
    3. Puts the values of {len_fifo_1} and {len_fifo_2} into {cell}.
    4. Then puts the answer of the computation back into {len}.
    4. Returns the group that does this.
    """
    cell = comp.add("cell", 32)
    with comp.group(group) as update_length_grp:
        cell.left = len_fifo_1.out
        cell.right = len_fifo_2.out
        len.write_en = 1
        len.in_ = cell.out
        update_length_grp.done = len.done
    return update_length_grp


def insert_pifo(prog):
    """A PIFO that achieves a 50/50 split between two flows.

    Up to the availability of values, this PIFO seeks to alternate 50/50
    between two "flows".

    We say "up to availability" because, if one flow is silent and the other
    is active, the active ones gets to emit consecutive values (in temporary
    violation of the 50/50 rule) until the silent flow starts transmitting again.
    At that point we go back to 50/50.

    Say the PIFO's maximum capacity is 10. Create two FIFOs, each of capacity 10.
    Let's say the two floww are called `1` and `2`, and our FIFOs are called
    `fifo_1` and `fifo_2`.
    Maintain additionally a register that points to which of these FIFOs is "hot".
    Start off with `hot` pointing to `fifo_1` (arbitrarily).
    Maintain `cold` that points to the other fifo.

    - len(PIFO) = len(fifo_1) + len(fifo_2)
    - `push(v, f, PIFO)`:
       + If len(PIFO) = 10, raise an "overflow" err and exit.
       + Otherwise, the charge is to enqueue value `v`, that is known to be from
         flow `f`, and `f` better be either `1` or `2`.
         Enqueue `v` into `fifo_f`.
         Note that the FIFO's enqueue method is itself partial: it may raise
         "overflow", in which case we propagate the overflow flag.
    - `pop(PIFO)`:
       + If `len(PIFO)` = 0, raise an "underflow" flag and exit.
       + Try `pop(FIFO_{hot})`.
         * If it succeeds it will return a value `v`; just propagate `v`. Also flip
           `hot` and `cold`.
         * If it fails because of underflow, return `pop(FIFO_{cold})`.
           Leave `hot` and `cold` as they were.
    """

    pifo: cb.ComponentBuilder = prog.new_component("pifo")
    pop = pifo.input("pop", 1)
    push = pifo.input("push", 1)
    payload = pifo.input("payload", 32)  # The value to push
    flow = pifo.input("flow", 1)  # The flow to push to

    ans = pifo.reg("ans", 32, is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`

    err = pifo.reg("err", 1, is_ref=True)
    # We'll raise this as a general error flag:
    # overflow,
    # underflow,
    # if the user calls pop and push at the same time,
    # or if the user issues no command.
    err_fifo_1 = pifo.reg("err_fifo_1", 1, is_ref=True)
    err_fifo_2 = pifo.reg("err_fifo_2", 1, is_ref=True)

    len = pifo.reg("len", 32, is_ref=True)  # The length of the PIFO
    len_fifo_1 = pifo.reg("len_fifo_1", 32, is_ref=True)  # The length of fifo_1
    len_fifo_2 = pifo.reg("len_fifo_2", 32, is_ref=True)  # The length of fifo_2

    # Create the two FIFOs.
    fifo_1 = fifo.insert_fifo(prog)
    fifo_2 = fifo.insert_fifo(prog)

    # Create the two registers.
    hot = pifo.register("hot", 1)
    cold = pifo.register("cold", 1)

    # Some equality checks.
    hot_eq_1 = util.insert_eq(pifo, hot, 1, "hot_eq_1", 1)  # hot == 1
    flow_eq_1 = util.insert_eq(pifo, flow, 1, "flow_eq_1", 1)  # flow == 1
    len_eq_0 = util.insert_eq(pifo, len.out, 0, "len_eq_0", 32)  # `len` == 0
    len_eq_10 = util.insert_eq(pifo, len.out, 10, "len_eq_10", 32)  # `len` == 10
    pop_eq_push = util.insert_eq(pifo, pop, push, "pop_eq_push", 1)  # `pop` == `push`
    pop_eq_1 = util.insert_eq(pifo, pop, 1, "pop_eq_1", 1)  # `pop` == 1
    push_eq_1 = util.insert_eq(pifo, push, 1, "push_eq_1", 1)  # `push` == 1
    err_1_eq_1 = util.insert_eq(pifo, err_fifo_1, 1, "err_1_eq_1", 1)  # err_1 == 1
    err_2_eq_1 = util.insert_eq(pifo, err_fifo_2, 1, "err_2_eq_1", 1)  # err_2 == 1

    swap = reg_swap(pifo, hot, cold, "swap")  # Swap `hot` and `cold`.
    raise_err = util.reg_store(pifo, err, 1, "raise_err")  # set `err` to 1
    zero_out_ans = util.reg_store(pifo, ans, 0, "zero_out_ans")  # zero out `ans`
    update_length = insert_len_update(
        pifo, len, len_fifo_1, len_fifo_2, "update_length"
    )

    # The main logic.
    pifo.control += [
        cb.if_(
            pop_eq_push[0].out,
            pop_eq_push[1],
            # Checking if the user called pop and push at the same time,
            # or issued no command.
            [
                raise_err,  # If so, we're done.
                zero_out_ans,  # We zero out the answer register.
            ],
            cb.par(  # If not, we continue.
                cb.if_(
                    # Did the user call pop?
                    pop_eq_1[0].out,
                    pop_eq_1[1],
                    cb.if_(
                        # Yes, the user called pop. But is the queue empty?
                        len_eq_0[0].out,
                        len_eq_0[1],
                        [raise_err, zero_out_ans],  # The queue is empty: underflow.
                        [  # The queue is not empty. Proceed.
                            # Check if `hot` is 1.
                            cb.if_(
                                hot_eq_1[0].out,
                                hot_eq_1[1],
                                [  # `hot` is 1. We'll invoke `pop` on `fifo_1`.
                                    cb.invoke(  # First we call pop
                                        fifo_1,
                                        in_pop=cb.const(1, 1),
                                        in_push=cb.const(1, 0),
                                        ref_ans=ans,  # Its answer is our answer.
                                        ref_err=err_fifo_1,  # We sequester its error.
                                        ref_len=len_fifo_1,
                                    ),
                                    # Now we check if `fifo_1` raised an error.
                                    cb.if_(
                                        err_1_eq_1[0].out,
                                        err_1_eq_1[1],
                                        [  # `fifo_1` raised an error.
                                            # We'll try to pop from `fifo_2`.
                                            cb.invoke(
                                                fifo_2,
                                                in_pop=cb.const(1, 1),
                                                in_push=cb.const(1, 0),
                                                ref_ans=ans,
                                                # Its answer is our answer.
                                                ref_err=err,
                                                # its error is our error
                                                ref_len=len_fifo_2,
                                            ),
                                        ],
                                        [  # `fifo_1` did not raise an error.
                                            # Its answer is our answer.
                                            # We'll just swap `hot` and `cold`.
                                            swap,
                                        ],
                                    ),
                                ],
                                [  # `hot` is 2.
                                    # We'll proceed symmetrically.
                                    cb.invoke(
                                        fifo_2,
                                        in_pop=cb.const(1, 1),
                                        in_push=cb.const(1, 0),
                                        ref_ans=ans,  # Its answer is our answer.
                                        ref_err=err_fifo_2,  # We sequester its error.
                                        ref_len=len_fifo_2,
                                    ),
                                    # Now we check if `fifo_2` raised an error.
                                    cb.if_(
                                        err_2_eq_1[0].out,
                                        err_2_eq_1[1],
                                        [  # `fifo_2` raised an error.
                                            # We'll try to pop from `fifo_1`.
                                            cb.invoke(
                                                fifo_1,
                                                in_pop=cb.const(1, 1),
                                                in_push=cb.const(1, 0),
                                                ref_ans=ans,
                                                # Its answer is our answer.
                                                ref_err=err,
                                                # its error is our error
                                                ref_len=len_fifo_1,
                                            ),
                                        ],
                                        [  # `fifo_2` did not raise an error.
                                            # Its answer is our answer.
                                            # We'll just swap `hot` and `cold`.
                                            swap,
                                        ],
                                    ),
                                ],
                            ),
                            update_length,  # Update the length of the PIFO.
                        ],
                    ),
                ),
                cb.if_(
                    # Did the user call push?
                    push_eq_1[0].out,
                    push_eq_1[1],
                    cb.if_(
                        # Yes, the user called push. But is the queue full?
                        len_eq_10[0].out,
                        len_eq_10[1],
                        [raise_err, zero_out_ans],  # The queue is full: overflow.
                        [  # The queue is not full. Proceed.
                            # TK
                            update_length,  # Update the length of the PIFO.
                        ],
                    ),
                ),
            ),
        )
    ]

    return pifo
