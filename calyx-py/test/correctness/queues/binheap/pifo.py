# pylint: disable=import-error
import sys
import calyx.builder as cb
import calyx.queue_call as qc
from stable_binheap import insert_stable_binheap


def insert_flow_inference(comp, value, flow, boundary, group):
    """The flow is needed when the command is a push.
    If the value to be pushed is less than or equal to `boundary`,
    the value belongs to flow 0.
    Otherwise, the value belongs to flow 1.

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


def insert_binheap_pifo(prog, name, boundary, factor):
    """Inserts the component `pifo` into the program.

    It is a two-flow, round-robin queue implemented via binary heap

    It has:
    - two inputs, `cmd` and `value`.
    - one memory, `mem`, of size `2**queue_size_factor`.
    - two ref registers, `ans` and `err`.
    """
    comp = prog.component(name)

    binheap = insert_stable_binheap(prog, "binheap", factor)
    binheap = comp.cell("binheap", binheap)

    cmd = comp.input("cmd", 2)
    value = comp.input("value", 32)

    ans = comp.reg(32, "ans", is_ref=True)
    err = comp.reg(1, "err", is_ref=True)

    cmd_eq_0 = comp.eq_use(cmd, 0)
    cmd_eq_2 = comp.eq_use(cmd, 2)

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
    turn_neq_flow_out = comp.neq_use(turn.out, flow_out.out)

    init = comp.reg(1, "init")
    init_eq_0 = comp.eq_use(init.out, 0)

    comp.control += [
        cb.if_with(init_eq_0, [comp.reg_store(r_b, 1), comp.incr(init)]),
        infer_flow_in,
        cb.if_(
            flow_in.out,
            cb.invoke(
                binheap,
                in_value=value,
                in_rank=r_b.out,
                in_cmd=cmd,
                ref_ans=ans,
                ref_err=err,
            ),
            cb.invoke(
                binheap,
                in_value=value,
                in_rank=r_a.out,
                in_cmd=cmd,
                ref_ans=ans,
                ref_err=err,
            ),
        ),
        infer_flow_out,
        cb.if_with(
            cmd_eq_0,
            cb.if_with(
                turn_neq_flow_out,
                cb.if_(flow_out.out, r_a_incr_2, r_b_incr_2),
                comp.incr(turn),
            )
        ),
        cb.if_with(
            cmd_eq_2, 
            cb.if_(
                flow_in.out, 
                r_b_incr_2, 
                r_a_incr_2
            )
        )
    ]

    return comp


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    prog = cb.Builder()
    pifo = insert_binheap_pifo(prog, "pifo", 200, 4)
    qc.insert_main(prog, pifo, num_cmds)
    return prog.program


if __name__ == "__main__":
    build().emit()
