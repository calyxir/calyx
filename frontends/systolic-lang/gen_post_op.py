#!/usr/bin/env python3

import calyx.builder as cb
from calyx import py_ast

# Global constant for the current bitwidth.
BITWIDTH = 32
INTWIDTH = 16
FRACWIDTH = 16
# Name of the ouput array
OUT_MEM = "out_mem"
DEFAULT_POST_OP = "default_post_op"
LEAKY_RELU_POST_OP = "leaky_relu_post_op"
from fud.stages.verilator import numeric_types
from calyx.utils import float_to_fixed_point


def add_register_params(comp: cb.ComponentBuilder, name):
    """
    Add params to component `comp` if we want to use a register named
    `name` inside `comp.`
    """
    comp.output(f"{name}_write_en", 1)
    comp.output(f"{name}_in", BITWIDTH)
    comp.input(f"{name}_out", BITWIDTH)


def add_write_mem_arguments(comp: cb.ComponentBuilder, name, addr_width):
    """
    Add arguments to component `comp` if we want to write to a mem named `name` with
    width of `addr_width` inside `comp.`
    """
    comp.output(f"{name}_addr0", addr_width)
    comp.output(f"{name}_write_data", BITWIDTH)
    comp.output(f"{name}_write_en", 1)
    comp.input(f"{name}_done", 1)


def add_systolic_input_params(comp: cb.ComponentBuilder, row_num, addr_width):
    """
    Add ports "r_{row_num}_valid", "r_{row_num}_value", "r_{row_num}_idx" to comp.
    These ports are meant to read from the systolic array.
    """
    comp.input(f"r{row_num}_valid", 1)
    comp.input(f"r{row_num}_value", BITWIDTH)
    comp.input(f"r{row_num}_idx", addr_width)


def create_write_mem_groups(comp: cb.ComponentBuilder, row_num):
    """
    Instantiates group that writes the systolic array values for `row_num` into
    memory.
    `comp` should have access to systolic array values through the parameters added
    in `add_systolic_input_params()`.
    """
    this = comp.this()
    # ports to write to memory
    write_en_port = this.port(OUT_MEM + f"_{row_num}_write_en")
    write_data_port = this.port(OUT_MEM + f"_{row_num}_write_data")
    addr0_port = this.port(OUT_MEM + f"_{row_num}_addr0")

    # ports to read from systolic array
    valid_port = this.port(f"r{row_num}_valid")
    value_port = this.port(f"r{row_num}_value")
    idx_port = this.port(f"r{row_num}_idx")

    # group that writes output of systolic arrays to memory
    with comp.static_group(f"write_r{row_num}", 1) as g:
        g.asgn(write_en_port, valid_port)
        g.asgn(write_data_port, value_port)
        g.asgn(addr0_port, idx_port)


def create_cond_reg_groups(comp: cb.ComponentBuilder, num_rows, num_cols, idx_width):
    """
    Writes to `cond_reg`, which should be initialized to hold 1.
    It should then hold 0 once we want to stop executing the while loop.
    """
    this = comp.this()
    # ports to read from systolic array
    valid_port = this.port(f"r{num_rows -1}_valid")
    idx_port = this.port(f"r{num_rows-1}_idx")
    cond_reg = comp.reg("cond_reg", 1)
    max_idx = num_cols - 1
    with comp.static_group("init_cond_reg", 1):
        cond_reg.in_ = 1
        cond_reg.write_en = 1
    with comp.static_group(f"write_cond_reg", 1):
        cond_reg.in_ = 0
        cond_reg.write_en = (
            valid_port
            & (idx_port == cb.ExprBuilder(py_ast.ConstantPort(idx_width, max_idx)))
        ) @ 1


def default_post_op(prog: cb.Builder, num_rows, num_cols, idx_width):
    """
    Adds a default post-op to `prog`.
    This post-op does nothing except immediately write to memory.
    """
    comp = prog.component(name=DEFAULT_POST_OP)
    for r in range(num_rows):
        add_write_mem_arguments(comp, OUT_MEM + f"_{r}", idx_width)
        add_systolic_input_params(comp, r, idx_width)
        create_write_mem_groups(comp, r)
    create_cond_reg_groups(comp, num_rows, num_cols, idx_width)

    while_body = py_ast.StaticParComp(
        [py_ast.Enable("write_cond_reg")]
        # write to memory
        + [py_ast.Enable(f"write_r{r}") for r in range(num_rows)]
    )
    while_loop = cb.while_(comp.get_cell("cond_reg").port("out"), while_body)
    comp.control = py_ast.SeqComp([py_ast.Enable("init_cond_reg"), while_loop])


def leaky_relu_comp(prog: cb.Builder):
    """
    Creates a dynamic, non-pipelined, leaky relu component
    """
    comp = prog.component(name="leaky_relu")
    comp.input("value", BITWIDTH)
    comp.input("index", BITWIDTH)
    # Takes a memory and register (i.e., arguments that essentially act as ref cells)
    add_write_mem_arguments(comp, OUT_MEM, BITWIDTH)
    add_register_params(comp, "idx_reg")

    this = comp.this()

    addr0_port = cb.ExprBuilder.unwrap(this.port(OUT_MEM + "_addr0"))
    write_data_port = cb.ExprBuilder.unwrap(this.port(OUT_MEM + "_write_data"))
    write_en_port = cb.ExprBuilder.unwrap(this.port(OUT_MEM + "_write_en"))
    write_done_port = this.port(OUT_MEM + "_done")

    fp_mult = comp.fp_sop("fp_mult", "mult_pipe", BITWIDTH, INTWIDTH, FRACWIDTH)
    lt = comp.fp_sop("val_lt", "lt", BITWIDTH, INTWIDTH, FRACWIDTH)
    incr_idx = comp.add(BITWIDTH, "incr_idx")
    write_mem = comp.wire("write_mem", 1)

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
        g.asgn(write_en_port, 1, write_mem.out)
        g.asgn(addr0_port, this.index)
        g.asgn(write_data_port, this.value, ~lt.out)
        g.asgn(write_data_port, fp_mult.out, lt.out)
        # Groups is done once we have written to memory.
        g.done = write_done_port

    comp.control = py_ast.Enable("do_relu")


def create_leaky_relu_groups(comp: cb.ComponentBuilder, row, num_cols, addr_width):
    """ """

    def store_output_vals(comp: cb.ComponentBuilder, row, num_cols, addr_width):
        this = comp.this()
        for col in range(num_cols):
            wire_value = comp.wire(f"r{row}_c{col}_val_wire", BITWIDTH)
            reg_value = comp.reg(f"r{row}_c{col}_val_reg", BITWIDTH)
            val_ready = comp.wire(f"r{row}_c{col}_val_ready", 1)
            valid_signal = this.port(f"r{row}_valid", 1)
            idx_signal = this.port(f"r{row}_idx", addr_width)
            value_signal = this.port(f"r{row}_signal", BITWIDTH)
            with comp.static_group(f"r{row}_c{col}_value_group", 1):
                val_ready.in_ = (valid_signal & (idx_signal == col)) @ 1
                wire_value.in_ = val_ready.out @ value_signal
                wire_value.in_ = ~(val_ready.out) @ reg_value.out
                reg_value.in_ = val_ready.out @ value_signal
                reg_value.write_en = val_ready.out @ 1

    # Helper function adds assignment wire.in = reg.out == col ? pe_{row}_{col}_out.
    def build_assignment(
        comp: cb.ComponentBuilder, group: cb.GroupBuilder, wire, register, output_val
    ):
        group.asgn(
            wire.port("in"),
            output_val.out,
            register.port("out") == cb.ExprBuilder(py_ast.ConstantPort(BITWIDTH, col)),
        )

    group = comp.static_group(f"r{row}_helper", 1)
    group_assigns = []

    # Current value we are performing relu on.
    cur_val = comp.wire(f"r{row}_cur_val", BITWIDTH)
    # Current idx within the row (i.e., column) for the value we are performing relu on.
    idx_reg = comp.reg(f"r{row}_cur_idx", BITWIDTH)
    # assigning cur_val = value of PE at (row,idx_reg).
    store_output_vals(comp, row, num_cols, addr_width)
    for col in range(num_cols):
        output_val = comp.cell(f"r{row}_c{col}_val_wire")
        group_assigns.append(
            build_assignment(comp, group, cur_val, idx_reg, output_val)
        )

    # instantiate an instance of a leaky_relu component
    relu_instance = comp.cell(f"leaky_relu_r{row}", py_ast.CompInst("leaky_relu", []))
    # Wire that tells us we are finished with relu operation for this row.
    relu_finished_wire = comp.wire(f"relu_finished_wire_r{row}", 1)

    # Annoying memory stuff because we can't use ref cells
    this = comp.this()
    mem_name = OUT_MEM + f"_{row}"
    addr0_port = cb.ExprBuilder.unwrap(this.port(mem_name + "_addr0"))
    relu_addr0_port = relu_instance.port(OUT_MEM + "_addr0")
    write_data_port = cb.ExprBuilder.unwrap(this.port(mem_name + "_write_data"))
    relu_write_data_port = relu_instance.port(OUT_MEM + "_write_data")
    write_en_port = cb.ExprBuilder.unwrap(this.port(mem_name + "_write_en"))
    relu_write_en_port = relu_instance.port(OUT_MEM + "_write_en")
    done_port = this.port(mem_name + "_done")
    relu_done_port = cb.ExprBuilder.unwrap(relu_instance.port(OUT_MEM + "_done"))

    with comp.static_group(f"execute_relu_r{row}", 1) as g:
        # Handle incrementing the idx_reg.
        relu_instance.go = (~relu_finished_wire.out) @ cb.ExprBuilder(
            py_ast.ConstantPort(1, 1)
        )
        # input ports
        relu_instance.value = cur_val.out
        relu_instance.index = idx_reg.out
        g.asgn(relu_done_port, done_port)
        relu_instance.idx_reg_out = idx_reg.out
        # output ports
        g.asgn(addr0_port, relu_addr0_port)
        g.asgn(write_data_port, relu_write_data_port)
        g.asgn(write_en_port, relu_write_en_port)
        idx_reg.write_en = relu_instance.idx_reg_write_en
        idx_reg.in_ = relu_instance.idx_reg_in

        relu_finished_wire.in_ = (
            idx_reg.out == cb.ExprBuilder(py_ast.ConstantPort(BITWIDTH, num_cols))
        ) @ 1


def leaky_relu_post_op(prog: cb.Builder):
    """
    Adds inefficient leaky relu post op to `prog`
    """
