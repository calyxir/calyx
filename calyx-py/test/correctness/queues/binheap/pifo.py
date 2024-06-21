# pylint: disable=import-error
import sys
import calyx.builder as cb
import calyx.queue_call as qc
from stable_binheap import insert_stable_binheap

FACTOR = 4
BOUNDARY = 200


def insert_flow_inference(comp, value, flow, boundary, group):
    """If the value to be pushed is less than or equal to `boundary`, the value
    belongs to flow A. Otherwise, the value belongs to flow B.

    This method adds a group to the component `comp` that does this.
    1. Within component `comp`, creates a group called `group`.
    2. Within `group`, creates a cell `cell` that checks for less-than.
    3. Puts the values `boundary` and `value` into the left and right ports of `cell`.
    4. Then puts the answer of the computation into `flow`.
    5. Returns the group that does this.
    """
    cell = comp.lt(32)
    with comp.group(group) as infer_flow_grp:
        cell.left = boundary
        cell.right = value
        flow.write_en = 1
        flow.in_ = cell.out
        infer_flow_grp.done = flow.done
    return infer_flow_grp


def insert_binheap_pifo(prog, name, boundary=BOUNDARY, queue_size_factor=FACTOR):
    """Inserts the component `pifo` into the program.

    It is a two-flow, round-robin queue implemented via binary heap.

    It has:
    - two inputs, `cmd` and `value`.
        - `cmd` has width 2.
        - `value` has width 32.
    - one memory, `mem`, of size `2**queue_size_factor`.
    - two ref registers, `ans` and `err`.
        - `ans` has width 32.
        - `err` has width 1.

    Call our flows A and B, represented by 0 and 1 respectively.
    When popping, we pop one value from A, one from B, and so on.
    If one class is silent and the other is active, we pop from the active class
    in FIFO order until the silent class starts up again. The erstwhile silent class
    does not get any form of "credit" for the time it was silent.

    We use `binheap`, a stable binary heap, rank pointers `r_a` and `r_b`, and a 1-bit
    signal `turn`. The pointers `r_a` and `r_b` represent the rank assigned to the
    next pushed value from flows A and B respectively. The signal `turn` stores
    which flow's turn it is.
    - `turn` is initialized to 0 (i.e. flow A).
    - `r_a` is initialized to 0.
    - `r_b` is initialized to 1.
    - To push value `v_a` (resp. `v_b`) from flow A (resp. B),
        - we push `(r_a, v_a)` (resp. `(r_b, v_b)`) to `binheap`.
        - Then, `r_a += 2` (resp. `r_b += 2`).
    - To pop, we pop `binheap`; say we obtain `v`.
        - if the flow `v` belongs to matches `turn`, switch `turn` to the other flow.
        - Otherwise, if `v` belongs to flow A (resp. B), `r_b += 2` (resp. `r_a += 2`).
    - To peek, we peek `binheap`.

    This mechanism for moving `r_a` and `r_b` ensures flows A and B are
    interleaved correctly.
    """
    comp = prog.component(name)

    binheap = insert_stable_binheap(prog, "binheap", queue_size_factor)
    binheap = comp.cell("binheap", binheap)

    cmd = comp.input("cmd", 2)
    value = comp.input("value", 32)

    ans = comp.reg(32, "ans", is_ref=True)
    err = comp.reg(1, "err", is_ref=True)

    cmd_eq_0 = comp.eq_use(cmd, 0)
    cmd_eq_2 = comp.eq_use(cmd, 2)
    err_eq_0 = comp.eq_use(err.out, 0)

    flow_in = comp.reg(1, "flow_in")
    infer_flow_in = insert_flow_inference(
        comp, value, flow_in, boundary, "infer_flow_in"
    )

    flow_out = comp.reg(1, "flow_out")
    infer_flow_out = insert_flow_inference(
        comp, ans.out, flow_out, boundary, "infer_flow_out"
    )

    r_a = comp.reg(32, "r_a")
    r_a_incr_2 = comp.incr(r_a, 2)

    r_b = comp.reg(32, "r_b")
    r_b_incr_2 = comp.incr(r_b, 2)

    turn = comp.reg(1, "turn")
    turn_eq_flow_out = comp.eq_use(turn.out, flow_out.out)

    init = comp.reg(1, "init")
    init_eq_0 = comp.eq_use(init.out, 0)

    def binheap_invoke_helper(value, rank):
        return cb.invoke(
            binheap,
            in_value=value,
            in_rank=rank,
            in_cmd=cmd,
            ref_ans=ans,
            ref_err=err,
        )

    comp.control += [
        cb.if_with(init_eq_0, [comp.reg_store(r_b, 1), comp.incr(init)]),
        infer_flow_in,
        cb.if_(
            flow_in.out,
            binheap_invoke_helper(value, r_b.out),
            binheap_invoke_helper(value, r_a.out),
        ),
        cb.if_with(
            err_eq_0,
            [
                cb.if_with(
                    cmd_eq_0,
                    [
                        infer_flow_out,
                        cb.if_with(
                            turn_eq_flow_out,
                            comp.incr(turn),
                            cb.if_(flow_out.out, r_a_incr_2, r_b_incr_2),
                        ),
                    ],
                ),
                cb.if_with(cmd_eq_2, cb.if_(flow_in.out, r_b_incr_2, r_a_incr_2)),
            ],
        ),
    ]

    return comp


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    pifo = insert_binheap_pifo(prog, "pifo")
    qc.insert_main(prog, pifo, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
