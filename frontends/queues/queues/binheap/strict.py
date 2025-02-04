# pylint: disable=import-error
import calyx.builder as cb
from calyx.utils import bits_needed
from queues.binheap.stable_binheap import insert_stable_binheap
from queues.flow_inference import insert_boundary_flow_inference

FACTOR = 4


def insert_binheap_strict(prog, name, n, order, flow_infer, queue_size_factor=FACTOR):
    comp = prog.component(name)

    binheap = insert_stable_binheap(prog, f"{name}_binheap", queue_size_factor)
    binheap = comp.cell("binheap", binheap)

    flow_infer = comp.cell("flow_infer", flow_infer)

    cmd = comp.input("cmd", 1)
    value = comp.input("value", 32)

    ans = comp.reg(32, "ans", is_ref=True)
    err = comp.reg(1, "err", is_ref=True)

    flow = comp.reg(bits_needed(n - 1), "flow")
    infer_flow = cb.invoke(flow_infer, in_value=value, ref_flow=flow)

    def binheap_invoke(value, rank):
        return cb.invoke(
            binheap,
            in_value=value,
            in_rank=cb.const(32, rank),
            in_cmd=cmd,
            ref_ans=ans,
            ref_err=err,
        )

    binheap_invokes = dict(
        [(i, binheap_invoke(value, order.index(i))) for i in range(n)]
    )

    comp.control += [infer_flow, comp.case(flow.out, binheap_invokes)]

    return comp
