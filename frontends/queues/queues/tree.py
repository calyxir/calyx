# pylint: disable=import-error
import calyx.builder as cb
from calyx.utils import bits_needed

def insert_tree(prog, name, root, children, flow_infer):
    comp = prog.component(name)

    flow_infer = comp.cell("flow_infer", flow_infer)

    root = comp.cell("root", root)
    children = [
        comp.cell(f"child_{i}", child) for i, child in enumerate(children)
    ]

    cmd = comp.input("cmd", 1)  
    value = comp.input("value", 32)  

    ans = comp.reg(32, "ans", is_ref=True)

    err = comp.reg(1, "err", is_ref=True)
    err_eq_0 = comp.eq_use(err.out, 0)

    flow = comp.reg(32, "flow")
    infer_flow = cb.invoke(flow_infer, in_value=value, ref_flow=flow)

    def invoke_child(child, cmd, value, ans, err):
        return cb.invoke(child, 
                         in_cmd=cmd, 
                         in_value=value, 
                         ref_ans=ans, 
                         ref_err=err)

    recurse = { n: invoke_child(children[n], cmd, value, ans, err) for n in range(len(children)) }

    push_logic = [
        infer_flow,
        cb.invoke(root, 
                  in_cmd=cmd, 
                  in_value=flow.out,
                  ref_ans=ans, 
                  ref_err=err),
        cb.if_with(
            err_eq_0,
            comp.case(flow.out, recurse)
        )
    ]

    pop_logic = [
        cb.invoke(root, 
                  in_cmd=cmd, 
                  in_value=flow.out, 
                  ref_ans=ans, 
                  ref_err=err),
        cb.if_with(
            err_eq_0, 
            comp.case(ans.out, recurse)
        )
    ]

    comp.control += comp.case(cmd, { 0: pop_logic, 1: push_logic })

    return comp
