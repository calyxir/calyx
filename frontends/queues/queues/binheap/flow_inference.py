# pylint: disable=import-error
import calyx.builder as cb
from calyx.utils import bits_needed

def insert_boundary_flow_inference(prog, name, boundaries):
    n = len(boundaries)
    
    comp = prog.component(name)

    value = comp.input("value", 32)
    flow = comp.reg(bits_needed(n - 1), "flow", is_ref=True)

    bound_checks = []
    for b in range(n):
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
    
    comp.control += [ cb.par(*bound_checks) ]

    return comp
