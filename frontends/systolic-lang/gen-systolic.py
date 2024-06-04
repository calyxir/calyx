#!/usr/bin/env python3
import calyx.builder as cb
from systolic_arg_parser import SystolicConfiguration
from calyx.utils import bits_needed
from gen_array_component import (
    create_systolic_array,
    BITWIDTH,
    NAME_SCHEME,
)
from gen_post_op import (
    default_post_op,
    relu_post_op,
    leaky_relu_post_op,
    relu_dynamic_post_op,
    OUT_MEM,
)

# Dict that maps command line arguments (e.g., "leaky-relu") to component names
# and function that creates them.
POST_OP_DICT = {
    None: default_post_op,
    "leaky-relu": leaky_relu_post_op,
    "relu": relu_post_op,
    "relu-dynamic": relu_dynamic_post_op,
}


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
    mem = main.comb_mem_d1(
        mem_name,
        BITWIDTH,
        mem_size,
        bits_needed(mem_size),
        is_external=True,
    )
    input_port_names = ["addr0"] if read_mem else ["write_data", "write_en", "addr0"]
    output_port_names = ["read_data"] if read_mem else ["done"]
    return cb.build_connections(
        mem, component_builder, "", f"{mem_name}_", input_port_names, output_port_names
    )


def build_main(prog, config: SystolicConfiguration, comp_unit, postop_comp):
    """
    Build the main component.
    It basically connects the ports of the systolic component and post_op component
    in a single group so that they run.
    """
    top_length, top_depth, left_length, left_depth = (
        config.top_length,
        config.top_depth,
        config.left_length,
        config.left_depth,
    )
    main = prog.component("main")
    systolic_array = main.cell("systolic_array_component", comp_unit)
    post_op = main.cell("post_op_component", postop_comp)
    # Connections contains the RTL-like connections between the ports of
    # systolic_array_comp and the post_op.
    # Also connects the input memories to the systolic_array_comp and
    # output memories to the post_op_component.
    connections = []
    # Connect input memories to systolic_array
    for r in range(top_length):
        connections += create_mem_connections(
            main,
            systolic_array,
            f"t{r}",
            top_depth,
            read_mem=True,
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
        # Connect output memory to post op. want to write to this memory.
        connections += create_mem_connections(
            main,
            post_op,
            OUT_MEM + f"_{i}",
            top_length,
            read_mem=False,
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
    systolic_done_reg = main.reg(1, "systolic_done")
    systolic_done_wire = main.wire("systolic_done_wire", 1)
    with main.group("perform_computation") as g:
        for i, o in connections:
            g.asgn(i, o)
        # Use systolic_done_wire to avoid retriggering systolic array after
        # it is done.
        systolic_done_reg.write_en = systolic_array.done @ 1
        systolic_done_reg.in_ = systolic_array.done @ 1
        systolic_done_wire.in_ = (systolic_array.done | systolic_done_reg.out) @ 1
        systolic_array.go = ~systolic_done_wire.out @ cb.HI
        systolic_array.depth = cb.const(BITWIDTH, left_depth)

        # Triggering post_op component.
        post_op.go = cb.HI
        # Group is done when post_op is done.
        g.done = post_op.computation_done

    main.control += g


if __name__ == "__main__":
    systolic_config = SystolicConfiguration()
    systolic_config.parse_arguments()
    # Building the main component
    prog = cb.Builder()
    comp_unit_inserted = create_systolic_array(prog, systolic_config)
    if systolic_config.post_op in POST_OP_DICT.keys():
        component_building_func = POST_OP_DICT[systolic_config.post_op]
        postop_comp_inserted = component_building_func(prog, config=systolic_config)
    else:
        raise ValueError(
            f"{systolic_config.post_op} not supported as a post op. \
                Supported post ops are (None means you pass no argument for -p) \
                {POST_OP_DICT.keys()}"
        )

    build_main(
        prog,
        config=systolic_config,
        comp_unit=comp_unit_inserted,
        postop_comp=postop_comp_inserted,
    )
    prog.program.emit()
