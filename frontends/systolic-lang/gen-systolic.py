#!/usr/bin/env python3
import calyx.builder as cb
from calyx.utils import bits_needed
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
)
from calyx import py_ast


def create_mem_connections(
    main: cb.ComponentBuilder,
    component_builder: cb.ComponentBuilder,
    mem_name: str,
    mem_size: int,
    read_mem: bool,
):
    """
    Instantiates 1d memory named mem_name with idx widths of idx_width.
    Also connects the memory ports to `component_builder`
    If `read_mem` == True, then connects memory ports such that
    `component_builder` can read from memory.
    If `read_mem` == False, then connects memory ports such that
    `component_builder` can write to memory.
    """
    mem = main.mem_d1(
        mem_name,
        BITWIDTH,
        mem_size,
        bits_needed(mem_size),
        is_external=True,
    )
    input_portname = ["addr0"] if read_mem else ["write_data", "write_en", "addr0"]
    output_portnames = ["read_data"] if read_mem else ["done"]
    return cb.build_connections(
        mem, component_builder, "", f"{mem_name}_", input_portname, output_portnames
    )


def build_main(
    prog, top_length, top_depth, left_length, left_depth, post_op_component_name
):
    """
    Build the main component.
    It basically connects the ports of the systolic component and post_op component
    in a single group so that they run.
    """
    assert top_depth == left_depth, (
        f"Cannot multiply matrices: "
        f"{top_length}x{top_depth} and {left_depth}x{left_length}"
    )
    main = prog.component("main")
    systolic_array = main.cell(
        "systolic_array_component", py_ast.CompInst(SYSTOLIC_ARRAY_COMP, [])
    )
    post_op = main.cell(
        "post_op_component", py_ast.CompInst(post_op_component_name, [])
    )
    # Connections contains the RTL-like connections between the ports of
    # systolic_array_comp and the post_op.
    # Also connects the input memories to the systolic_array_comp and
    # output memories to the post_op_component.
    connections = []
    # Connect input memories to systolic_array
    for r in range(top_length):
        connections += create_mem_connections(
            main, systolic_array, f"t{r}", top_depth, read_mem=True
        )
    for c in range(left_length):
        connections += create_mem_connections(
            # top_depth should = left_depth
            main,
            systolic_array,
            f"l{c}",
            left_depth,
            read_mem=True,
        )
    # Connect outout memories to post_op, and systolic_array_output to
    # post_op inputs.
    for i in range(left_length):
        # connect output memory to post op
        connections += create_mem_connections(
            main, post_op, OUT_MEM + f"_{i}", top_length, read_mem=False
        )
        # Connect systolic array to post op
        connections += cb.build_connections(
            post_op,
            systolic_array,
            "",
            "",
            [
                NAME_SCHEME["systolic valid signal"].format(row_num=i),
                NAME_SCHEME["systolic value signal"].format(row_num=i),
                NAME_SCHEME["systolic idx signal"].format(row_num=i),
            ],
            [],
        )
    # Use a wire and register so that we have a signal that tells us when
    # systolic array component is done. This way, we don't retrigger systolic_array_comp
    # when it has already finished.
    systolic_done_reg = main.reg("systolic_done", 1)
    systolic_done_wire = main.wire("systolic_done_wire", 1)
    with main.group("perform_computation") as g:
        for i, o in connections:
            g.asgn(i, o)
        # Use systolic_done_wire to avoid retriggering systolic array after
        # it is done.
        systolic_done_reg.write_en = systolic_array.done @ 1
        systolic_done_reg.in_ = systolic_array.done @ 1
        systolic_done_wire.in_ = (systolic_array.done | systolic_done_reg.out) @ 1
        systolic_array.go = ~systolic_done_wire.out @ py_ast.ConstantPort(1, 1)
        systolic_array.depth = py_ast.ConstantPort(BITWIDTH, left_depth)

        # Triggering post_op component.
        post_op.go = py_ast.ConstantPort(1, 1)
        # Group is done when post_op is done.
        g.done = post_op.computation_done

    main.control = py_ast.Enable("perform_computation")


def parse_arguments() -> (int, int, int, int, bool):
    """
    Parses arguments and returns the following outputs:
    top_length, top_depth, left_length, left_depth, leaky_relu
    """
    import argparse
    import json

    # Arg parsing
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
    return (top_length, top_depth, left_length, left_depth, leaky_relu)


if __name__ == "__main__":
    (top_length, top_depth, left_length, left_depth, leaky_relu) = parse_arguments()
    # Building the main component
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
            prog,
            num_rows=left_length,
            num_cols=top_length,
            idx_width=bits_needed(top_length),
        )
        post_op_component_name = LEAKY_RELU_POST_OP
    else:
        default_post_op(
            prog,
            num_rows=left_length,
            num_cols=top_length,
            idx_width=bits_needed(top_length),
        )
        post_op_component_name = DEFAULT_POST_OP
    build_main(
        prog,
        top_length=top_length,
        top_depth=top_depth,
        left_length=left_length,
        left_depth=left_depth,
        post_op_component_name=post_op_component_name,
    )
    prog.program.emit()
