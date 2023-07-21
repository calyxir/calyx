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

    len = pifo.reg("len", 32, is_ref=True)  # The length of the queue

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
    len_incr = util.insert_incr(pifo, len, "add5", "len_incr")  # len++
    len_decr = util.insert_decr(pifo, len, "add6", "len_decr")  # len--
    pop_eq_push = util.insert_eq(pifo, pop, push, "pop_eq_push", 1)  # `pop` == `push`
    pop_eq_1 = util.insert_eq(pifo, pop, 1, "pop_eq_1", 1)  # `pop` == 1
    push_eq_1 = util.insert_eq(pifo, push, 1, "push_eq_1", 1)  # `push` == 1

    swap = reg_swap(pifo, hot, cold, "swap")  # Swap `hot` and `cold`.
    raise_err = util.reg_store(pifo, err, 1, "raise_err")  # set `err` to 1
    zero_out_ans = util.reg_store(pifo, ans, 0, "zero_out_ans")  # zero out `ans`

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
                            # TK
                            len_decr,  # Decrement the length.
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
                            len_incr,  # Increment the length.
                        ],
                    ),
                ),
            ),
        )
    ]

    return pifo
