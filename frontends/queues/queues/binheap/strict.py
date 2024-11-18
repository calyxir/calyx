# pylint: disable=import-error
import calyx.builder as cb
from calyx.utils import bits_needed
from queues.binheap.stable_binheap import insert_stable_binheap
from queues.binheap.flow_inference import insert_flow_inference

FACTOR = 4


def insert_binheap_strict(prog, name, boundaries, order, queue_size_factor=FACTOR):
    n = len(boundaries)

    comp = prog.component(name)

    binheap = insert_stable_binheap(prog, "binheap", queue_size_factor)
    binheap = comp.cell("binheap", binheap)

    cmd = comp.input("cmd", 1)
    value = comp.input("value", 32)

    ans = comp.reg(32, "ans", is_ref=True)
    err = comp.reg(1, "err", is_ref=True)

    flow = comp.reg(bits_needed(n - 1), "flow")
    infer_flow = insert_flow_inference(comp, value, flow, boundaries, "infer_flow")

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


def generate(prog, numflows):
    """Generates queue with specific `boundaries`"""

    if numflows == 2:
        boundaries = [200, 400]
        order = [1, 0]
    elif numflows == 3:
        boundaries = [133, 266, 400]
        order = [1, 2, 0]
    elif numflows == 4:
        boundaries = [100, 200, 300, 400]
        order = [3, 0, 2, 1]
    elif numflows == 5:
        boundaries = [80, 160, 240, 320, 400]
        order = [0, 1, 2, 3, 4]
    elif numflows == 6:
        boundaries = [66, 100, 200, 220, 300, 400]
        order = [3, 1, 5, 2, 4, 0]
    elif numflows == 7:
        boundaries = [50, 100, 150, 200, 250, 300, 400]
        order = [0, 1, 2, 3, 4, 5, 6]
    else:
        raise ValueError("Unsupported number of flows")

    pifo = insert_binheap_strict(prog, "pifo", boundaries, order)

    return pifo
