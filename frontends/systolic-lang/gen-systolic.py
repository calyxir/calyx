#!/usr/bin/env python3
import calyx.builder as cb
from gen_array_component import (
    create_systolic_array,
    BITWIDTH,
    SYSTOLIC_ARRAY_COMP,
    NAME_SCHEME,
)
from gen_post_op import (
    default_post_op,
    leaky_relu_post_op,
    OUT_MEM,
    DEFAULT_POST_OP,
    LEAKY_RELU_POST_OP,
    COND_REG,
)
from calyx import py_ast
from calyx.utils import bits_needed


def instantiate_cond_reg(comp: cb.ComponentBuilder):
    cond_reg = comp.get_cell(COND_REG)
    with comp.static_group("init_cond_reg", 1):
        cond_reg.in_ = 1
        cond_reg.write_en = 1


def build_main(prog, post_op_name):
    """
    Build the main component.
    It basically connects the ports of the systolic component and post_op component
    so that they both run.
    """
    main = prog.component("main")
    systolic_array = main.cell(
        "systolic_array_component", py_ast.CompInst(SYSTOLIC_ARRAY_COMP, [])
    )
    post_op = main.cell("post_op_component", py_ast.CompInst(post_op_name, []))
    cond_reg = main.reg(COND_REG, 1)
    instantiate_cond_reg(main)
    # Connections contains the RTL-like connections between the ports of
    # systolic_array_comp and the post_op.
    connections = []
    # connect input memories to systolic_array
    for r in range(top_length):
        name = f"t{r}"
        idx_width = bits_needed(top_depth)
        mem = main.mem_d1(
            name,
            BITWIDTH,
            top_depth,
            idx_width,
            is_external=True,
        )
        connections.append((systolic_array.port(f"{name}_read_data"), mem.read_data))
        connections.append((mem.addr0, systolic_array.port(f"{name}_addr0")))
    for c in range(left_length):
        name = f"l{c}"
        idx_width = bits_needed(left_depth)
        mem = main.mem_d1(
            name,
            BITWIDTH,
            left_depth,
            idx_width,
            is_external=True,
        )
        connections.append((systolic_array.port(f"{name}_read_data"), mem.read_data))
        connections.append((mem.addr0, systolic_array.port(f"{name}_addr0")))
    # connect outout memories to post_op
    for i in range(left_length):
        name = OUT_MEM + f"_{i}"
        mem = main.mem_d1(
            name,
            BITWIDTH,
            top_length,
            BITWIDTH,
            is_external=True,
        )
        connections.append((mem.addr0, post_op.port(f"{name}_addr0")))
        connections.append((mem.write_data, post_op.port(f"{name}_write_data")))
        connections.append((mem.write_en, post_op.port(f"{name}_write_en")))
        connections.append((post_op.port(f"{name}_done"), mem.done))
        connections.append(
            (
                post_op.port(NAME_SCHEME["systolic valid signal"].format(row_num=i)),
                systolic_array.port(
                    NAME_SCHEME["systolic valid signal"].format(row_num=i)
                ),
            )
        )
        connections.append(
            (
                post_op.port(NAME_SCHEME["systolic value signal"].format(row_num=i)),
                systolic_array.port(
                    NAME_SCHEME["systolic value signal"].format(row_num=i)
                ),
            )
        )
        connections.append(
            (
                post_op.port(NAME_SCHEME["systolic idx signal"].format(row_num=i)),
                systolic_array.port(
                    NAME_SCHEME["systolic idx signal"].format(row_num=i)
                ),
            )
        )
    systolic_array_done = main.reg("systolic_done", 1)
    systolic_done_wire = main.wire("systolic_done_wire", 1)
    post_op_cond_reg_write_en = post_op.port(f"{COND_REG}_write_en")
    post_op_cond_reg_in = post_op.port(f"{COND_REG}_in")
    post_op_cond_reg_out = post_op.port(f"{COND_REG}_out")
    with main.static_group("perform_computation", 1) as g:
        for i, o in connections:
            g.asgn(i, o)
        systolic_array.go = ~systolic_done_wire.out @ py_ast.ConstantPort(1, 1)
        systolic_array_done.write_en = systolic_array.done @ 1
        systolic_array_done.in_ = systolic_array.done @ 1
        systolic_done_wire.in_ = (systolic_array.done | systolic_array_done.out) @ 1
        post_op.go = py_ast.ConstantPort(1, 1)
        systolic_array.go = py_ast.ConstantPort(1, 1)
        systolic_array.depth = py_ast.ConstantPort(BITWIDTH, left_depth)
        g.asgn(post_op_cond_reg_out, cond_reg.out)
        g.asgn(cond_reg.write_en, post_op_cond_reg_write_en)
        g.asgn(cond_reg.in_, post_op_cond_reg_in)

    while_loop = cb.while_(cond_reg.port("out"), py_ast.Enable("perform_computation"))
    main.control = py_ast.SeqComp([py_ast.Enable("init_cond_reg"), while_loop])


if __name__ == "__main__":
    import argparse
    import json

    parser = argparse.ArgumentParser(description="Process some integers.")
    parser.add_argument("file", nargs="?", type=str)
    parser.add_argument("-tl", "--top-length", type=int)
    parser.add_argument("-td", "--top-depth", type=int)
    parser.add_argument("-ll", "--left-length", type=int)
    parser.add_argument("-ld", "--left-depth", type=int)
    parser.add_argument("-r", "--leaky-relu", action="store_true")

    args = parser.parse_args()

    top_length, top_depth, left_length, left_depth, leaky_relu = (
        None,
        None,
        None,
        None,
        False,
    )

    fields = [args.top_length, args.top_depth, args.left_length, args.left_depth]
    if all(map(lambda x: x is not None, fields)):
        top_length = args.top_length
        top_depth = args.top_depth
        left_length = args.left_length
        left_depth = args.left_depth
        leaky_relu = args.leaky_relu
    elif args.file is not None:
        with open(args.file, "r") as f:
            spec = json.load(f)
            top_length = spec["top_length"]
            top_depth = spec["top_depth"]
            left_length = spec["left_length"]
            left_depth = spec["left_depth"]
            # default to not perform leaky_relu
            leaky_relu = spec.get("leaky_relu", False)
    else:
        parser.error(
            "Need to pass either `FILE` or all of `"
            "-tl TOP_LENGTH -td TOP_DEPTH -ll LEFT_LENGTH -ld LEFT_DEPTH`"
        )

    prog = cb.Builder()
    create_systolic_array(
        prog,
        top_length=top_length,
        top_depth=top_depth,
        left_length=left_length,
        left_depth=left_depth,
    )
    if leaky_relu:
        leaky_relu_post_op(
            prog, num_rows=left_length, num_cols=top_length, idx_width=BITWIDTH
        )
        post_op = LEAKY_RELU_POST_OP
    else:
        default_post_op(
            prog, num_rows=left_length, num_cols=top_length, idx_width=BITWIDTH
        )
        post_op = DEFAULT_POST_OP
    build_main(prog, post_op)
    prog.program.emit()
