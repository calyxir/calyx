# pylint: disable=import-error
import calyx.builder as cb
from calyx.utils import bits_needed
from calyx.tuple import insert_untuplify


def insert_boundary_flow_inference(prog, name, boundaries, flow_bits=None):
    flow_bits = bits_needed(len(boundaries) - 1) if flow_bits is None else flow_bits

    comp = prog.component(name)

    value = comp.input("value", 32)
    flow = comp.reg(flow_bits, "flow", is_ref=True)

    bound_checks = []
    for b in range(len(boundaries)):
        lt = comp.lt(32)
        le = comp.le(32)
        guard = comp.and_(1)

        with comp.comb_group(f"{name}_bound_check_{b}") as bound_check_b:
            le.left = value
            le.right = boundaries[b]
            if b > 0:
                lt.left = boundaries[b - 1]
                lt.right = value
            else:
                lt.left = 0
                lt.right = 1
            guard.left = le.out
            guard.right = lt.out

        set_flow_b = comp.reg_store(flow, b, groupname=f"set_flow_{b}")

        bound_check = cb.if_with(cb.CellAndGroup(guard, bound_check_b), set_flow_b)
        bound_checks.append(bound_check)

    comp.control += [cb.par(*bound_checks)]

    return comp


def insert_tuple_flow_inference(prog, name, num_flows):
    flow_bits = bits_needed(num_flows - 1)

    comp = prog.component(name)

    untuplify = insert_untuplify(prog, f"{name}_untuplify", flow_bits, 32 - flow_bits)
    untuplify = comp.cell("untuplify", untuplify)

    value = comp.input("value", 32)
    flow = comp.reg(flow_bits, "flow", is_ref=True)

    with comp.continuous:
        untuplify.tup = value

    comp.control += [
        comp.reg_store(flow, untuplify.fst),
        comp.reg_store(flow, flow.out, "nop"),  # temporary no-op to compile
        # question about this on Zulip:
        # https://calyx.zulipchat.com/#narrow/channel/423433-general/topic/Circular.20Combinational.20Logic/near/479748231
    ]

    return comp


def insert_value_flow_inference(prog, name, num_flows):
    flow_bits = bits_needed(num_flows - 1)

    comp = prog.component(name)

    untuplify = insert_untuplify(prog, f"{name}_untuplify", 32 - flow_bits, flow_bits)
    untuplify = comp.cell("untuplify", untuplify)

    value = comp.input("value", 32)
    flow = comp.reg(flow_bits, "flow", is_ref=True)

    with comp.continuous:
        untuplify.tup = value

    comp.control += [
        comp.reg_store(flow, untuplify.snd),
        comp.reg_store(flow, flow.out, "nop"),  # temporary no-op to compile
        # question about this on Zulip:
        # https://calyx.zulipchat.com/#narrow/channel/423433-general/topic/Circular.20Combinational.20Logic/near/479748231
    ]

    return comp
