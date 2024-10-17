# pylint: disable=import-error
import calyx.builder as cb
from calyx.utils import bits_needed
from queues.binheap.stable_binheap import insert_stable_binheap
from queues.binheap.flow_inference import insert_flow_inference

FACTOR = 4


def insert_binheap_rr(prog, name, boundaries, queue_size_factor=FACTOR):
    n = len(boundaries)

    comp = prog.component(name)

    binheap = insert_stable_binheap(prog, "binheap", queue_size_factor)
    binheap = comp.cell("binheap", binheap)

    cmd = comp.input("cmd", 1)
    value = comp.input("value", 32)

    ans = comp.reg(32, "ans", is_ref=True)
    err = comp.reg(1, "err", is_ref=True)

    err_eq_0 = comp.eq_use(err.out, 0)

    flow_in = comp.reg(bits_needed(n - 1), "flow_in")
    infer_flow_in = insert_flow_inference(
        comp, value, flow_in, boundaries, "infer_flow_in"
    )

    flow_out = comp.reg(bits_needed(n - 1), "flow_out")
    infer_flow_out = insert_flow_inference(
        comp, ans.out, flow_out, boundaries, "infer_flow_out"
    )

    rank_ptrs = [comp.reg(32, f"r_{i}") for i in range(n)]
    rank_ptr_incrs = dict([(i, comp.incr(rank_ptrs[i], n)) for i in range(n)])

    turn = comp.reg(bits_needed(n - 1), "turn")
    turn_neq_flow_out = comp.neq_use(turn.out, flow_out.out)
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
            turn_neq_flow_out, [comp.case(turn.out, rank_ptr_incrs), turn_incr_mod_n]
        ),
        turn_incr_mod_n,
    ]
    update_state_push = comp.case(flow_in.out, rank_ptr_incrs)

    comp.control += [
        init_state,
        infer_flow_in,
        comp.case(flow_in.out, binheap_invokes),
        cb.if_with(
            err_eq_0, comp.case(cmd, {0: update_state_pop, 1: update_state_push})
        ),
    ]

    return comp


def generate(prog, numflows):
    """Generates queue with specific `boundaries`"""

    if numflows == 2:
        boundaries = [200, 400]
    elif numflows == 3:
        boundaries = [133, 266, 400]
    elif numflows == 4:
        boundaries = [100, 200, 300, 400]
    elif numflows == 5:
        boundaries = [80, 160, 240, 320, 400]
    elif numflows == 6:
        boundaries = [66, 100, 200, 220, 300, 400]
    elif numflows == 7:
        boundaries = [50, 100, 150, 200, 250, 300, 400]
    else:
        raise ValueError("Unsupported number of flows")

    pifo = insert_binheap_rr(prog, "pifo", boundaries)

    return pifo
