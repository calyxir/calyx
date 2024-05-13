#!/usr/bin/env python3

from calyx import builder as cb
from calyx import py_ast
from gen_array_component import NAME_SCHEME
from gen_pe import BITWIDTH, INTWIDTH, FRACWIDTH
from calyx import numeric_types
from calyx.utils import float_to_fixed_point
from systolic_arg_parser import SystolicConfiguration
from calyx.utils import bits_needed


# Name of the ouput array
OUT_MEM = "out_mem"
DEFAULT_POST_OP = "default_post_op"
RELU_POST_OP = "relu_post_op"
LEAKY_RELU_POST_OP = "leaky_relu_post_op"
RELU_DYNAMIC_POST_OP = "relu_dynamic_post_op"
COND_REG = "cond_reg"
WRITE_DONE_COND = "write_done_cond"


def add_systolic_input_params(comp: cb.ComponentBuilder, row_num, addr_width):
    """
    Add ports "r_{row_num}_valid", "r_{row_num}_value", "r_{row_num}_idx" to comp.
    These ports are meant to read from the systolic array output.
    """
    cb.add_comp_params(
        comp,
        input_ports=[
            (NAME_SCHEME["systolic valid signal"].format(row_num=row_num), 1),
            (NAME_SCHEME["systolic value signal"].format(row_num=row_num), BITWIDTH),
            (NAME_SCHEME["systolic idx signal"].format(row_num=row_num), addr_width),
        ],
        output_ports=[],
    )


def add_post_op_params(comp: cb.Builder, num_rows: int, idx_width: int):
    """
    Adds correct parameters for post op component comp
    """
    comp.output("computation_done", 1)
    for r in range(num_rows):
        cb.add_write_mem_params(
            comp, OUT_MEM + f"_{r}", data_width=BITWIDTH, addr_width=idx_width
        )
        add_systolic_input_params(comp, r, idx_width)


def create_immediate_done_condition(
    comp: cb.ComponentBuilder,
    num_rows: int,
    num_cols: int,
    idx_width: int,
):
    """
    Creates wiring for an "immediate" done condition.
    In other words, the done condition is triggered the cycle after the
    systolic array presents its final value.
    """
    this = comp.this()
    final_row_valid = this.port(f"r{num_rows -1}_valid")
    final_row_idx = this.port(f"r{num_rows-1}_idx")
    max_idx = num_cols - 1
    # delay_reg delays writing to this.computation_done
    delay_reg = comp.reg(1, "delay_reg")
    final_col_idx_reached = final_row_idx == cb.ExprBuilder(
        py_ast.ConstantPort(idx_width, max_idx)
    )
    with comp.static_group(WRITE_DONE_COND, 1):
        delay_reg.in_ = 1
        # When we are at the final index in the final row, we still need
        # one cycle to write into the memory. Therefore, we delay computation_done
        # by one cycle.
        delay_reg.write_en = (final_row_valid & final_col_idx_reached) @ 1
        this.computation_done = delay_reg.done @ 1


def imm_write_mem_groups(comp: cb.ComponentBuilder, row_num: int, perform_relu: bool):
    """
    Instantiates group that writes the systolic array values for `row_num` into
    the output memory.
    `comp` should have access to systolic array values through the parameters added
    in `add_systolic_input_params()`.
    """
    this = comp.this()
    # ports to write to memory
    write_en_port = this.port(OUT_MEM + f"_{row_num}_write_en")
    write_data_port = this.port(OUT_MEM + f"_{row_num}_write_data")
    addr0_port = this.port(OUT_MEM + f"_{row_num}_addr0")

    # ports to read from systolic array
    valid_port = this.port(NAME_SCHEME["systolic valid signal"].format(row_num=row_num))
    value_port = this.port(NAME_SCHEME["systolic value signal"].format(row_num=row_num))
    idx_port = this.port(NAME_SCHEME["systolic idx signal"].format(row_num=row_num))

    if perform_relu:
        # lt operator to see if value is < 0
        lt = comp.fp_sop(f"val_lt_r{row_num}", "lt", BITWIDTH, INTWIDTH, FRACWIDTH)
        # group that writes output of systolic arrays to memory
        with comp.static_group(f"write_r{row_num}", 1) as g:
            lt.left = value_port
            lt.right = 0
            g.asgn(write_en_port, valid_port)
            g.asgn(write_data_port, value_port, ~lt.out)
            g.asgn(write_data_port, 0, lt.out)
            g.asgn(addr0_port, idx_port)
    else:
        with comp.static_group(f"write_r{row_num}", 1) as g:
            g.asgn(write_en_port, valid_port)
            g.asgn(write_data_port, value_port)
            g.asgn(addr0_port, idx_port)


def imm_write_mem_post_op(
    prog: cb.Builder, config: SystolicConfiguration, perform_relu: bool
) -> cb.ComponentBuilder:
    """
    This post-op does nothing except immediately write to memory.
    If perform_relu is true, then writes 0 to memory if result < 0, and writes
    the result to memory otherwise. In other words, it performs relu on the value
    before writing to memory.
    """
    (num_rows, num_cols) = config.get_output_dimensions()
    idx_width = bits_needed(num_cols)
    post_op_name = RELU_POST_OP if perform_relu else DEFAULT_POST_OP
    comp = prog.component(name=post_op_name)
    add_post_op_params(comp, num_rows, idx_width)
    for r in range(num_rows):
        imm_write_mem_groups(comp, r, perform_relu=perform_relu)
    create_immediate_done_condition(comp, num_rows, num_cols, idx_width)

    comp.control = py_ast.StaticParComp(
        [py_ast.Enable(WRITE_DONE_COND)]
        # write to memory
        + [py_ast.Enable(f"write_r{r}") for r in range(num_rows)]
    )

    return comp


def default_post_op(prog: cb.Builder, config: SystolicConfiguration):
    """
    Default post op that immediately writes to output memory.
    """
    return imm_write_mem_post_op(prog=prog, config=config, perform_relu=False)


def relu_post_op(prog: cb.Builder, config: SystolicConfiguration):
    """
    Relu post op that (combinationally) performs relu before
    immediately writing the result to memory.
    """
    return imm_write_mem_post_op(prog=prog, config=config, perform_relu=True)


def add_dynamic_op_params(comp: cb.ComponentBuilder, idx_width: int):
    """
    Adds neccesary parameters for dynamic ops, including:
    1) Input value
    1) Parameters to write the result of the op to memory.
    2) Input index (for the memory to write to)
    """
    comp.input("value", BITWIDTH)
    comp.input("idx", idx_width)
    cb.add_write_mem_params(comp, OUT_MEM, data_width=BITWIDTH, addr_width=idx_width)


def leaky_relu_comp(prog: cb.Builder, idx_width: int) -> cb.ComponentBuilder:
    """
    Creates a dynamic, non-pipelined, leaky relu component.
    This is the component that actually performs the leaky relu computation on
    a given output.
    """
    comp = prog.component(name="leaky_relu_op")
    add_dynamic_op_params(comp, idx_width)

    this = comp.this()

    fp_mult = comp.fp_sop("fp_mult", "mult_pipe", BITWIDTH, INTWIDTH, FRACWIDTH)
    lt = comp.fp_sop("val_lt", "lt", BITWIDTH, INTWIDTH, FRACWIDTH)
    write_mem = comp.wire("should_write_mem", 1)

    with comp.continuous:
        # gt holds whether this.value > 0
        lt.left = this.value
        lt.right = 0

    with comp.group("do_relu") as g:
        # Write_mem holds whether we should be writing to memory, which is when:
        # a) multiplier is done, so we write fp_mult.out to mem
        # b) this.value >=0 (i.e., !(this.value < 0)) so we write this.value to mem
        write_mem.in_ = (fp_mult.done | ~lt.out) @ 1
        # Trigger the multiplier when we're not writing to memory.
        fp_mult.left = numeric_types.FixedPoint(
            str(float_to_fixed_point(0.01, FRACWIDTH)), BITWIDTH, INTWIDTH, True
        ).unsigned_integer()
        fp_mult.right = this.value
        fp_mult.go = ~(write_mem.out) @ 1

        # Write to memory.
        this.out_mem_write_en = write_mem.out @ 1
        this.out_mem_addr0 = this.idx
        # Write value if this.value >= 0
        # Write mult.out if this.value < 0
        this.out_mem_write_data = ~lt.out @ this.value
        this.out_mem_write_data = lt.out @ fp_mult.out
        g.done = this.out_mem_done

    comp.control += g

    return comp


def relu_dynamic_comp(prog: cb.Builder, idx_width: int):
    """
    Creates a dynamic, regular RELU component.
    This dynamic implementation is meant to be compared to a static
    ReLU implementation in order to show the benefits of static groups and
    control.
    """
    comp = prog.component(name="relu_dynamic_op")
    add_dynamic_op_params(comp, idx_width)

    this = comp.this()

    lt = comp.fp_sop("val_lt", "lt", BITWIDTH, INTWIDTH, FRACWIDTH)

    with comp.continuous:
        # gt holds whether this.value > 0
        lt.left = this.value
        lt.right = 0

    with comp.group("do_relu") as g:
        # Write to memory.
        this.out_mem_write_en = 1
        this.out_mem_addr0 = this.idx
        # Write value if this.value >= 0
        # Write mult.out if this.value < 0
        this.out_mem_write_data = ~lt.out @ this.value
        this.out_mem_write_data = lt.out @ 0

        # It takes one cycle to write to g
        g.done = this.out_mem_done

    comp.control += g

    return comp


def generate_dynamic_post_op_done(comp: cb.ComponentBuilder, num_rows: int):
    """
    The done condition for leaky relu components is triggered once all of the
    leaky relu operations have finished.
    """
    this = comp.this()
    # Check if all relu operations have finished for each row
    guard = comp.get_cell("op_finished_wire_r0").out
    for r in range(1, num_rows):
        guard = guard & comp.get_cell(f"op_finished_wire_r{r}").out
    all_row_finished_wire = comp.wire("all_row_finished_wire", 1)
    with comp.static_group(WRITE_DONE_COND, 1):
        all_row_finished_wire.in_ = guard @ 1
        this.computation_done = all_row_finished_wire.out @ 1


def create_dynamic_post_op_groups(
    comp: cb.ComponentBuilder,
    row: int,
    num_cols: int,
    addr_width: int,
    op_component: cb.ComponentBuilder,
):
    """
    Creates the groups for the leaky relu post op, i.e., the post-op that
    coordinates the execution of the leaky relu component.
    """

    def store_output_vals(comp: cb.ComponentBuilder, row, num_cols, addr_width):
        """
        Helper function that looks at the systolic array output signals (e.g.,
        `r0_valid`, `r0_value`, etc.) and creates signals that tells us when: a)
        each row is ready for the leaky relu operations to start and b)
        the output systolic array values (we need them in registers bc the systolic
        array outputs are only available for one cycle).
        """
        this = comp.this()
        row_ready_wire = comp.wire(f"r{row}_ready_wire", 1)
        row_ready_reg = comp.reg(1, f"r{row}_ready_reg")
        for col in range(num_cols):
            wire_value = comp.wire(f"r{row}_c{col}_val_wire", BITWIDTH)
            reg_value = comp.reg(BITWIDTH, f"r{row}_c{col}_val_reg")
            val_ready = comp.wire(f"r{row}_c{col}_val_ready", 1)
            valid_signal = this.port(f"r{row}_valid")
            idx_signal = this.port(f"r{row}_idx")
            value_signal = this.port(f"r{row}_value")
            value_ready_signal = valid_signal & (
                idx_signal == cb.ExprBuilder(py_ast.ConstantPort(addr_width, col))
            )
            with comp.continuous:
                # Wire to detect and hold when the row is first valid. Once
                # it is valid, we can safely start our relu operations.
                row_ready_reg.in_ = valid_signal @ 1
                row_ready_reg.write_en = valid_signal @ 1
                row_ready_wire.in_ = (valid_signal | row_ready_reg.out) @ 1
                # Logic to hold the systolic array output values. We need registers
                # because the output values for systolic arrays are only available
                # for one cycle before they change.
                val_ready.in_ = value_ready_signal @ 1
                wire_value.in_ = val_ready.out @ value_signal
                wire_value.in_ = ~(val_ready.out) @ reg_value.out
                reg_value.in_ = val_ready.out @ value_signal
                reg_value.write_en = val_ready.out @ 1

    # Helper function adds assignment wire.in = reg.out == col ? pe_{row}_{col}_out.
    def build_assignment(wire, register, output_val):
        comp.continuous.asgn(
            wire.port("in"),
            output_val.out,
            register.port("out") == cb.const(BITWIDTH, col),
        )

    # Current value we are performing relu on.
    cur_val = comp.wire(f"r{row}_cur_val", BITWIDTH)
    # Current idx within the row (i.e., column) for the value we are performing relu on.
    idx_reg = comp.reg(addr_width, f"r{row}_cur_idx")
    # Handling logic to hold the systolic array's output values so they're available
    # for more than one cycle.
    store_output_vals(comp, row, num_cols, addr_width)
    for col in range(num_cols):
        output_val = comp.get_cell(f"r{row}_c{col}_val_wire")
        # Assigning to cur_val wire so that we always have the current value of the
        # row based on idx_reg.
        build_assignment(cur_val, idx_reg, output_val)

    # Instantiate an instance of a leaky_relu component
    op_instance = comp.cell(
        f"{op_component.component.name}_r{row}",
        py_ast.CompInst(op_component.component.name, []),
    )
    # Wire that tells us we are finished with relu operation for this row.
    row_finished_wire = comp.wire(f"op_finished_wire_r{row}", 1)
    row_ready_wire = comp.get_cell(f"r{row}_ready_wire")
    incr_idx = comp.add(bits_needed(num_cols), f"incr_idx_r{row}")

    # Need to pass this component's memory ports another layer down to
    # the leaky_relu cell.
    this_relu_io_ports = cb.build_connections(
        cell1=comp.this(),
        cell2=op_instance,
        root1=OUT_MEM + f"_{row}_",
        root2=OUT_MEM + "_",
        forward_ports=["addr0", "write_data", "write_en"],
        reverse_ports=["done"],
    )
    idx_limit_reached = idx_reg.out == cb.ExprBuilder(
        py_ast.ConstantPort(BITWIDTH, num_cols)
    )
    with comp.static_group(f"execute_relu_r{row}", 1) as g:
        for i, o in this_relu_io_ports:
            g.asgn(i, o)
        # Handle incrementing the idx_reg.
        incr_idx.left = idx_reg.out
        incr_idx.right = 1
        idx_reg.in_ = incr_idx.out
        # Increment idx once the op is done executing
        idx_reg.write_en = op_instance.done @ 1

        op_instance.go = (
            row_ready_wire.out & (~row_finished_wire.out) & (~op_instance.done)
        ) @ cb.HI
        # input ports for relu_instance
        op_instance.value = cur_val.out
        op_instance.idx = idx_reg.out
        row_finished_wire.in_ = idx_limit_reached @ 1


def dynamic_post_op(
    prog: cb.Builder,
    config: SystolicConfiguration,
    post_op_component_name: str,
    op_component: cb.ComponentBuilder,
):
    """
    Adds a dynamic post op that performs handles the coordination so that
    `op_component` (which can be dynamic) gets executed dynamically on each
    systolic array output.
    """
    num_rows, num_cols = config.get_output_dimensions()
    idx_width = bits_needed(num_cols)
    # Create a leaky relu component.
    comp = prog.component(name=post_op_component_name)
    add_post_op_params(comp, num_rows, idx_width)
    for r in range(num_rows):
        create_dynamic_post_op_groups(comp, r, num_cols, idx_width, op_component)
    generate_dynamic_post_op_done(comp, num_rows)

    # all_groups go in one big static par.
    all_groups = [py_ast.Enable(WRITE_DONE_COND)]
    for r in range(num_rows):
        all_groups.append(py_ast.Enable(f"execute_relu_r{r}"))

    comp.control = py_ast.StaticParComp(all_groups)

    return comp


def leaky_relu_post_op(prog: cb.Builder, config: SystolicConfiguration):
    _, num_cols = config.get_output_dimensions()
    leaky_relu_op_comp = leaky_relu_comp(prog, idx_width=bits_needed(num_cols))
    return dynamic_post_op(
        prog=prog,
        config=config,
        post_op_component_name=LEAKY_RELU_POST_OP,
        op_component=leaky_relu_op_comp,
    )


def relu_dynamic_post_op(prog: cb.Builder, config: SystolicConfiguration):
    _, num_cols = config.get_output_dimensions()
    relu_dynamic_op_comp = relu_dynamic_comp(prog, idx_width=bits_needed(num_cols))
    return dynamic_post_op(
        prog=prog,
        config=config,
        post_op_component_name=RELU_DYNAMIC_POST_OP,
        op_component=relu_dynamic_op_comp,
    )
