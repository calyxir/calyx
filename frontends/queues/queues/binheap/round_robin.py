# pylint: disable=import-error
import calyx.builder as cb
from calyx.utils import bits_needed
from queues.binheap.stable_binheap import insert_stable_binheap

FACTOR = 4


def insert_binheap_rr(prog, name, n, flow_infer, queue_size_factor=FACTOR):
    comp = prog.component(name)

    binheap = insert_stable_binheap(prog, f"{name}_binheap", queue_size_factor)
    binheap = comp.cell("binheap", binheap)

    flow_infer = comp.cell("flow_infer", flow_infer)

    cmd = comp.input("cmd", 1)
    value = comp.input("value", 32)

    ans = comp.reg(32, "ans", is_ref=True)
    err = comp.reg(1, "err", is_ref=True)

    err_eq_0 = comp.eq_use(err.out, 0)

    flow = comp.reg(bits_needed(n - 1), "flow")
    infer_flow_in = cb.invoke(flow_infer, in_value=value, ref_flow=flow)
    infer_flow_out = cb.invoke(flow_infer, in_value=ans.out, ref_flow=flow)

    rank_ptrs = [comp.reg(32, f"r_{i}") for i in range(n)]
    rank_ptr_incrs = dict([(i, comp.incr(rank_ptrs[i], n)) for i in range(n)])

    turn = comp.reg(bits_needed(n - 1), "turn")
    turn_neq_flow = comp.neq_use(turn.out, flow.out)
    turn_incr_mod_n = cb.if_with(
        comp.eq_use(turn.out, n - 1), comp.reg_store(turn, 0), comp.incr(turn)
    )

    init = comp.reg(1, "init")
    init_eq_0 = comp.eq_use(init.out, 0)
    init_state = cb.if_with(
        init_eq_0,
        [cb.par(*[comp.reg_store(rank_ptrs[i], i) for i in range(n)]), comp.incr(init)],
    )

    def binheap_invoke(value, rank):
        return cb.invoke(
            binheap,
            in_value=value,
            in_rank=rank,
            in_cmd=cmd,
            ref_ans=ans,
            ref_err=err,
        )

    binheap_invokes = dict(
        [(i, binheap_invoke(value, rank_ptrs[i].out)) for i in range(n)]
    )

    update_state_pop = [
        infer_flow_out,
        cb.while_with(
            turn_neq_flow, [comp.case(turn.out, rank_ptr_incrs), turn_incr_mod_n]
        ),
        turn_incr_mod_n,
    ]
    update_state_push = comp.case(flow.out, rank_ptr_incrs)

    comp.control += [
        init_state,
        infer_flow_in,
        comp.case(flow.out, binheap_invokes),
        cb.if_with(
            err_eq_0, comp.case(cmd, {0: update_state_pop, 1: update_state_push})
        ),
    ]

    return comp
