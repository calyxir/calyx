# pylint: disable=import-error
import pifo
import builder_util as util
import calyx.builder as cb


def insert_pifo_tree(prog, pifo_1, pifo_2):
    """A PIFO tree that achieves a 50/50 split between two flows, and
    further 50/50 split between its first flow.

    This is achieved by maintaining three PIFOs:
    - `pifo_0`: a PIFO that contains indices 1 or 2.
    - `pifo_1`: a PIFO that contains values from flow 1, i.e. 0-100.
      it split flow 1 further into two flows, flow 3 (0-50) and flow 4 (51-100),
      and gives them equal priority.
    - `pifo_2`: a PIFO that contains values from flow 2, i.e. 101-200.


    - len(pifo_tree) = len(pifo_0)
    - `push(v, f, pifotree)`:
       + If len(pifotree) = 10, raise an "overflow" err and exit.
       + Otherwise, the charge is to enqueue value `v`, that is known to be from
         flow `f`, and `f` better be `2`, `3`, or `4`.
         Enqueue `v` into `pifo_1` if `f` is `3` or `4`, and into `pifo_2` otherwise.
         If enqueueing into `pifo_1`, also enqueue the index `1` into `pifo_0`.
          If enqueueing into `pifo_2`, also enqueue the index `2` into `pifo_0`.
         Note that the PIFO's enqueue method is itself partial: it may raise
         "overflow", in which case we propagate the overflow flag.
    - `pop(pifotree)`:
       + If `len(pifotree)` = 0, raise an "underflow" flag and exit.
       + Perform pop(pifo_0). It will return an index `i` that is either 1 or 2.
         Perform pop(pifo_i). It will return a value `v`. Propagate `v`.
    """

    pifotree: cb.ComponentBuilder = prog.component("pifotree")
    pop = pifotree.input("pop", 1)
    push = pifotree.input("push", 1)
    payload = pifotree.input("payload", 32)  # The value to push
    flow = pifotree.input("flow", 2)  # The flow to push to

    ans = pifotree.reg("ans", 32, is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`

    err = pifotree.reg("err", 1, is_ref=True)
    # We'll raise this as a general error flag:
    # overflow,
    # underflow,
    # if the user calls pop and push at the same time,
    # or if the user issues no command.
    len = pifotree.reg("len", 32, is_ref=True)  # The length of the PIFO

    # Create the two PIFOs and ready them for invocation.
    pifo_1 = pifotree.cell("mypifo_1", pifo_1)
    pifo_2 = pifotree.cell("mypifo_2", pifo_2)

    # Some equality checks.
    flow_eq_2 = util.insert_eq(pifotree, flow, 2, "flow_eq_2", 2)  # flow == 2
    flow_eq_3 = util.insert_eq(pifotree, flow, 3, "flow_eq_3", 2)  # flow == 3
    flow_eq_4 = util.insert_eq(pifotree, flow, 4, "flow_eq_4", 2)  # flow == 4
    ans_eq_1 = util.insert_eq(pifotree, ans, 1, "ans_eq_1", 32)  # ans == 1
    ans_eq_2 = util.insert_eq(pifotree, ans, 2, "ans_eq_2", 32)  # ans == 2

    len_eq_0 = util.insert_eq(pifotree, len.out, 0, "len_eq_0", 32)  # `len` == 0
    len_eq_10 = util.insert_eq(pifotree, len.out, 10, "len_eq_10", 32)  # `len` == 10

    pop_eq_push = util.insert_eq(
        pifotree, pop, push, "pop_eq_push", 1
    )  # `pop` == `push`
    pop_eq_1 = util.insert_eq(pifotree, pop, 1, "pop_eq_1", 1)  # `pop` == 1
    push_eq_1 = util.insert_eq(pifotree, push, 1, "push_eq_1", 1)  # `push` == 1

    raise_err = util.reg_store(pifotree, err, 1, "raise_err")  # set `err` to 1
    zero_out_ans = util.reg_store(pifotree, ans, 0, "zero_out_ans")  # zero out `ans`

    len_incr = util.insert_incr(pifotree, len, "add1", "len_incr")  # len++
    len_decr = util.insert_decr(pifotree, len, "add2", "len_decr")  # len--

    # The main logic.
    pifotree.control += [
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
                        # Yes, the user called pop. But is the tree empty?
                        len_eq_0[0].out,
                        len_eq_0[1],
                        [raise_err, zero_out_ans],  # The tree is empty: underflow.
                        [  # The tree is not empty. Proceed.
                            # Pop the root PIFO.
                            cb.invoke(
                                pifo_0,
                                in_pop=cb.const(1, 1),
                                in_push=cb.const(1, 0),
                                ref_ans=ans,
                                ref_err=err,
                                ref_len=len,
                            ),
                            # Now we need to check which flow the root PIFO returned.
                            # is `ans` = 1 or 2?
                            cb.if_(
                                ans_eq_1[0].out,
                                ans_eq_1[1],
                                [  # `ans` is 1. We must pop from `pifo_1`.
                                    cb.invoke(
                                        pifo_1,
                                        in_pop=cb.const(1, 1),
                                        in_push=cb.const(1, 0),
                                        ref_ans=ans,  # Its answer is our answer.
                                        ref_err=err,  # Its error is our error.
                                        ref_len=len,
                                    ),
                                ]
                                ,
                                [
                                    # `ans` is 2. We must pop from `pifo_2`.
                                    cb.invoke(
                                        pifo_2,
                                        in_pop=cb.const(1, 1),
                                        in_push=cb.const(1, 0),
                                        ref_ans=ans,  # Its answer is our answer.
                                        ref_err=err,  # Its error is our error.
                                        ref_len=len,
                                    ),
                                ]
                            ),
                            len_decr,  # Update the length of the PIFO.
                        ],
                    ),
                ),
                cb.if_(
                    # Did the user call push?
                    push_eq_1[0].out,
                    push_eq_1[1],
                    cb.if_(
                        # Yes, the user called push. But is the tree full?
                        len_eq_10[0].out,
                        len_eq_10[1],
                        [raise_err, zero_out_ans],  # The tree is full: overflow.
                        [  # The tree is not full. Proceed.
                            # We need to check which flow the user wants to push to.
                            cb.if_(
                                flow_eq_2[0].out,
                                flow_eq_2[1],
                                [
                                  # The user wants to push to flow 2.
                                  cb.invoke(
                                      pifo_2,
                                      in_pop=cb.const(1, 0),
                                      in_push=cb.const(1, 1),
                                      in_payload=payload,
                                      in_flow=flow,
                                      ref_err=err,  # Its error is our error.
                                      ref_len=len,
                                      ref_ans=ans,
                                  ),
                                  # And we must also push the index 2 into `pifo_0`.
                                  cb.invoke(
                                      pifo_0,
                                      in_pop=cb.const(1, 0),
                                      in_push=cb.const(1, 1),
                                      in_payload=cb.const(32, 2),
                                      in_flow=cb.const(2, 2),
                                      ref_err=err,  # Its error is our error.
                                      ref_len=len,
                                      ref_ans=ans,
                                  ),
                                ],
                                [
                                  # The user wants to push to flow 3 or 4.
                                  # Which is it?
                                  cb.if_(
                                      flow_eq_3[0].out,
                                      flow_eq_3[1],
                                        # The user wants to push to flow 3.
                                        cb.invoke(
                                            pifo_1,
                                            in_pop=cb.const(1, 0),
                                            in_push=cb.const(1, 1),
                                            in_payload=payload,
                                            in_flow=flow,
                                            ref_err=err,  # Its error is our error.
                                            ref_len=len,
                                            ref_ans=ans,
                                        ),
                                      # The user wants to push to flow 4.
                                        cb.invoke(
                                            pifo_1,
                                            in_pop=cb.const(1, 0),
                                            in_push=cb.const(1, 1),
                                            in_payload=payload,
                                            in_flow=flow,
                                            ref_err=err,  # Its error is our error.
                                            ref_len=len,
                                            ref_ans=ans,
                                        ),
                                  )
                                  # Regardless, we must also push the index 1 into `pifo_0`.
                                  cb.invoke(
                                    piifo_0,
                                    in_pop=cb.const(1, 0),
                                    in_push=cb.const(1, 1),
                                    in_payload=cb.const(32, 1),
                                    in_flow=cb.const(2, 3),
                                    ref_err=err,  # Its error is our error.
                                    ref_len=len,
                                    ref_ans=ans,
                                  ),
                                ],
                            ),
                            len_incr,  # Update the length of the PIFO.
                        ],
                    ),
                ),
            ),
        )
    ]

    return pifotree
