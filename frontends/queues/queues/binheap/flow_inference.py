# pylint: disable=import-error
import calyx.builder as cb


def insert_flow_inference(comp, value, flow, boundaries, name):
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

        set_flow_b = comp.reg_store(flow, b, f"{name}_set_flow_{b}")
        bound_check = cb.if_with(cb.CellAndGroup(guard, bound_check_b), set_flow_b)

        bound_checks.append(bound_check)

    return cb.par(*bound_checks)
