#!/usr/bin/env python3
import calyx.builder as cb
from gen_array_component import create_systolic_array, BITWIDTH, OUT_MEM
from calyx.utils import bits_needed
from gen_post_op import default_post_op


def build_main(systolic_component, post_op_component):
    """
    Build the main component.
    It basically connects the ports of the systolic component and post_op component
    so that they both run.
    """
    main = prog.component("main")
    systolic_array = main.cell("systolic_array_component", systolic_component)
    post_op = main.cell("post_op_component", post_op_component)
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
        connections.append((systolic_array.port(mem.read_data, f"{name}_read_data")))
    for col in range(left_length):
        name = f"l{col}"
        idx_width = bits_needed(left_depth)
        mem = main.mem_d1(
            name,
            BITWIDTH,
            left_depth,
            idx_width,
            is_external=True,
        )
        connections.append((systolic_array.port(f"{name}_read_data"), mem.read_data))
        connections.append((systolic_array.port(mem.read_data, f"{name}_read_data")))
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
        connections.append(mem.addr0, post_op.port(f"{name}_addr0"))
        connections.append(mem.write_data, post_op.port(f"{name}_write_data"))
        connections.append(mem.write_en, post_op.port(f"{name}_write_en"))
        connections.append(post_op.port(f"{name}_done"), mem.done)
    main.control = []


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
    default_post_op(prog, num_rows=left_length, num_cols=top_length, idx_width=BITWIDTH)
    prog.program.emit()
