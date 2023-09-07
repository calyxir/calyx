#!/usr/bin/env python3

import calyx.builder as cb
from calyx import py_ast
from gen_array_component import NAME_SCHEME
from gen_pe import BITWIDTH, INTWIDTH, FRACWIDTH
from fud.stages.verilator import numeric_types
from calyx.utils import float_to_fixed_point


# Name of the ouput array
OUT_MEM = "out_mem"
DEFAULT_POST_OP = "default_post_op"
LEAKY_RELU_POST_OP = "leaky_relu_post_op"
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


def create_write_mem_groups(comp: cb.ComponentBuilder, row_num):
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

    # group that writes output of systolic arrays to memory
    with comp.static_group(f"write_r{row_num}", 1) as g:
        g.asgn(write_en_port, valid_port)
        g.asgn(write_data_port, value_port)
        g.asgn(addr0_port, idx_port)


def done_condition_groups(
    comp: cb.ComponentBuilder,
    num_rows: int,
    num_cols: int,
    idx_width: int,
    leaky_relu: bool,
):
    """
    Writes to this.computation_done
    For leaky relu, we wait until the relu operations are done for each row.
    For default post op, you simply have to check when the systolic array output
    is at the last entry.
    """
    this = comp.this()
    # ports to read from systolic array
    if leaky_relu:
        # Check if all relu operations have finished for each row
        guard = comp.get_cell("relu_finished_wire_r0").out
        for r in range(1, num_rows):
            guard = guard & comp.get_cell(f"relu_finished_wire_r{r}").out
        all_relu_finished_wire = comp.wire("all_relu_finished_wire", 1)
        with comp.static_group(WRITE_DONE_COND, 1):
            all_relu_finished_wire.in_ = guard @ 1
            this.computation_done = all_relu_finished_wire.out @ 1
    else:
        final_row_valid = this.port(f"r{num_rows -1}_valid")
        final_row_idx = this.port(f"r{num_rows-1}_idx")
        max_idx = num_cols - 1
        # delay_reg delays writing to this.computation_done
        delay_reg = comp.reg("delay_reg", 1)
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


def default_post_op(prog: cb.Builder, num_rows, num_cols, idx_width):
    """
    Adds a default post-op to `prog`.
    This post-op does nothing except immediately write to memory.
    """
    comp = prog.component(name=DEFAULT_POST_OP)
    add_post_op_params(comp, num_rows, idx_width)
    for r in range(num_rows):
        create_write_mem_groups(comp, r)
    done_condition_groups(comp, num_rows, num_cols, idx_width, leaky_relu=False)

    comp.control = py_ast.StaticParComp(
        [py_ast.Enable(WRITE_DONE_COND)]
        # write to memory
        + [py_ast.Enable(f"write_r{r}") for r in range(num_rows)]
    )


def leaky_relu_comp(prog: cb.Builder, idx_width: int):
    """
    Creates a dynamic, non-pipelined, leaky relu component.
    This is the component that actually performs the leaky relu computation on
    a given output.
    """
    comp = prog.component(name="leaky_relu")
    comp.input("value", BITWIDTH)
    # Takes a memory and register (i.e., arguments that essentially act as ref cells)
    cb.add_write_mem_params(comp, OUT_MEM, data_width=BITWIDTH, addr_width=idx_width)
    cb.add_register_params(comp, "idx_reg", idx_width)

    this = comp.this()

    fp_mult = comp.fp_sop("fp_mult", "mult_pipe", BITWIDTH, INTWIDTH, FRACWIDTH)
    lt = comp.fp_sop("val_lt", "lt", BITWIDTH, INTWIDTH, FRACWIDTH)
    incr_idx = comp.add(idx_width, "incr_idx")
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

        # Increment idx_reg during the cycle that we write to memory.
        incr_idx.left = this.idx_reg_out
        incr_idx.right = 1
        this.idx_reg_in = write_mem.out @ incr_idx.out
        this.idx_reg_write_en = write_mem.out @ 1

        # Write to memory.
        this.out_mem_write_en = write_mem.out @ 1
        this.out_mem_addr0 = this.idx_reg_out
        # Write value if this.value >= 0
        # Write mult.out if this.value < 0
        this.out_mem_write_data = ~lt.out @ this.value
        this.out_mem_write_data = lt.out @ fp_mult.out
        g.done = this.out_mem_done

    comp.control = py_ast.Enable("do_relu")


def create_leaky_relu_groups(comp: cb.ComponentBuilder, row, num_cols, addr_width):
    """
    Creates the groups for the leaky relu post op, i.e., the post-op that
    coordinates the execution of the leaky relu component.
    """

    def store_output_vals(comp: cb.ComponentBuilder, row, num_cols, addr_width):
        """
        Helper function that looks at the systolic array output signsl (e.g.,
        `r0_valid`, `r0_value`, etc.) and creates signals that tells us when: a)
        each row is ready for the leaky relu operations to start and b)
        the output systolic array values (we need them in registers bc the systolic
        array outputs are only available for one cycle).
        """
        this = comp.this()
        row_ready_wire = comp.wire(f"r{row}_ready_wire", 1)
        row_ready_reg = comp.reg(f"r{row}_ready_reg", 1)
        for col in range(num_cols):
            wire_value = comp.wire(f"r{row}_c{col}_val_wire", BITWIDTH)
            reg_value = comp.reg(f"r{row}_c{col}_val_reg", BITWIDTH)
            val_ready = comp.wire(f"r{row}_c{col}_val_ready", 1)
            valid_signal = this.port(f"r{row}_valid")
            idx_signal = this.port(f"r{row}_idx")
            value_signal = this.port(f"r{row}_value")
            value_ready_signal = valid_signal & (
                idx_signal == cb.ExprBuilder(py_ast.ConstantPort(addr_width, col))
            )
            with comp.static_group(f"r{row}_c{col}_value_group", 1):
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
    def build_assignment(group: cb.GroupBuilder, wire, register, output_val):
        group.asgn(
            wire.port("in"),
            output_val.out,
            register.port("out") == cb.ExprBuilder(py_ast.ConstantPort(BITWIDTH, col)),
        )

    group = comp.static_group(f"r{row}_helper", 1)

    # Current value we are performing relu on.
    cur_val = comp.wire(f"r{row}_cur_val", BITWIDTH)
    # Current idx within the row (i.e., column) for the value we are performing relu on.
    idx_reg = comp.reg(f"r{row}_cur_idx", addr_width)
    # Handling logic to hold the systolic array's output values so they're available
    # for moer than one cycle.
    store_output_vals(comp, row, num_cols, addr_width)
    for col in range(num_cols):
        output_val = comp.get_cell(f"r{row}_c{col}_val_wire")
        # Assigning to cur_val wire so that we always have the current value of the
        # row based on idx_reg.
        build_assignment(group, cur_val, idx_reg, output_val)

    # Instantiate an instance of a leaky_relu component
    relu_instance = comp.cell(f"leaky_relu_r{row}", py_ast.CompInst("leaky_relu", []))
    # Wire that tells us we are finished with relu operation for this row.
    relu_finished_wire = comp.wire(f"relu_finished_wire_r{row}", 1)
    row_ready_wire = comp.get_cell(f"r{row}_ready_wire")

    # Need to pass this component's memory ports another layer down to
    # the leaky_relu cell.
    this_relu_io_ports = cb.build_connections(
        cell1=comp.this(),
        cell2=relu_instance,
        root1=OUT_MEM + f"_{row}_",
        root2=OUT_MEM + "_",
        forward_ports=["addr0", "write_data", "write_en"],
        reverse_ports=["done"],
    )
    # Building connections between relu and idx_reg
    relu_idx_io_ports = cb.build_connections(
        cell1=idx_reg,
        cell2=relu_instance,
        root1="",
        root2="idx_reg_",
        forward_ports=["write_en", "in"],
        reverse_ports=["out", "done"],
    )
    idx_limit_reached = idx_reg.out == cb.ExprBuilder(
        py_ast.ConstantPort(BITWIDTH, num_cols)
    )
    with comp.static_group(f"execute_relu_r{row}", 1) as g:
        for i, o in this_relu_io_ports:
            g.asgn(i, o)
        for i, o in relu_idx_io_ports:
            g.asgn(i, o)
        # Handle incrementing the idx_reg.
        relu_instance.go = (
            row_ready_wire.out & (~relu_finished_wire.out)
        ) @ cb.ExprBuilder(py_ast.ConstantPort(1, 1))
        # input ports for relu_instance
        relu_instance.value = cur_val.out
        relu_finished_wire.in_ = idx_limit_reached @ 1


def leaky_relu_post_op(prog: cb.Builder, num_rows, num_cols, idx_width):
    """
    Adds a dynamic leaky relu post op to `prog`
    """
    # Create a leaky relu component.
    leaky_relu_comp(prog, idx_width)
    comp = prog.component(name=LEAKY_RELU_POST_OP)
    add_post_op_params(comp, num_rows, idx_width)
    for r in range(num_rows):
        create_leaky_relu_groups(comp, r, num_cols, idx_width)
    done_condition_groups(comp, num_rows, num_cols, idx_width, leaky_relu=True)

    # all_groups go in one big static par.
    all_groups = [py_ast.Enable(WRITE_DONE_COND)]
    for r in range(num_rows):
        all_groups.append(py_ast.Enable(f"r{r}_helper"))
        all_groups.append(py_ast.Enable(f"execute_relu_r{r}"))
        for c in range(num_cols):
            all_groups.append(py_ast.Enable(f"r{r}_c{c}_value_group"))

    comp.control = py_ast.StaticParComp(all_groups)
