#!/usr/bin/env python3

import numpy as np
import calyx.builder as cb
from calyx import py_ast
from calyx.utils import bits_needed
from fud.stages.verilator import numeric_types
from calyx.utils import float_to_fixed_point

# Global constant for the current bitwidth.
BITWIDTH = 32
INTWIDTH = 16
FRACWIDTH = 16
# Name of the ouput array
OUT_MEM = "out_mem"
PE_NAME = "mac_pe"
DEPTH = "depth"

# Naming scheme for generated groups. Used to keep group names consistent
# across structure and control.
NAME_SCHEME = {
    # Indexing into the memory
    "index name": "{prefix}_idx",
    "index init": "{prefix}_idx_init",
    "index update": "{prefix}_idx_update",
    # Move data from main memories
    "memory move": "{prefix}_move",
    "out mem move": "{pe}_out_write",
    # Move data between internal registers
    "register move down": "{pe}_down_move",
    "register move right": "{pe}_right_move",
}


def add_write_mem_arguments(comp: cb.ComponentBuilder, name, addr_width):
    """
    Add arguments to component `comp` if we want to write to a mem named `name` with
    width of `addr_width` inside `comp.`
    """
    comp.output(f"{name}_addr0", addr_width)
    comp.output(f"{name}_write_data", BITWIDTH)
    comp.output(f"{name}_write_en", 1)
    comp.input(f"{name}_done", 1)


def add_systolic_input_arguments(comp: cb.ComponentBuilder, row_num, addr_width):
    """
    Add input arguments to that are meant to read from systolic array interface.
    Adds arguments to component `comp` for row `row_num`.
    """
    comp.input(f"r{row_num}_valid", 1)
    comp.input(f"r{row_num}_value", BITWIDTH)
    comp.input(f"r{row_num}_idx", addr_width)


def instantiate_write_mem(comp: cb.ComponentBuilder, row_num, addr_width):
    """
    Add input arguments to that are meant to read from systolic array interface.
    Adds arguments to component `comp` for row `row_num`.
    """
    this = comp.this()
    # ports to write to memory
    write_en_port = cb.ExprBuilder.unwrap(this.port(OUT_MEM + f"_{row_num}_write_en"))
    write_data_port = cb.ExprBuilder.unwrap(
        this.port(OUT_MEM + f"_{row_num}_write_data")
    )
    addr0_port = cb.ExprBuilder.unwrap(this.port(OUT_MEM + f"_{row_num}_addr0"))

    # ports to read from systolic array
    valid_port = this.port(f"r{row_num}_valid")
    value_port = this.port(f"r{row_num}_value")
    idx_port = this.port(f"r{row_num}_idx")

    # group that writes output of systolic arrays to memory
    with comp.static_group(f"write_r{row_num}", 1) as g:
        g.asgn(write_en_port, valid_port)
        g.asgn(write_data_port, value_port)
        g.asgn(addr0_port, idx_port)


def instantiate_cond_reg_groups(
    comp: cb.ComponentBuilder, num_rows, num_cols, idx_width
):
    """
    Writes to `cond_reg`
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
    Default post-op operation (that basically does nothing).
    """
    comp = prog.component(name="post_op")
    for r in range(num_rows):
        add_write_mem_arguments(comp, OUT_MEM + f"_{r}", idx_width)
        add_systolic_input_arguments(comp, r, idx_width)
        instantiate_write_mem(comp, r, idx_width)
    instantiate_cond_reg_groups(comp, num_rows, num_cols, idx_width)

    while_body = py_ast.StaticParComp(
        [py_ast.Enable("write_cond_reg")]
        + [py_ast.Enable(f"write_row_r{r}") for r in range(num_rows)]
    )
    while_loop = cb.while_(comp.get_cell("cond_reg").port("out"), while_body)
    comp.control = py_ast.SeqComp([py_ast.Enable("init_cond_reg"), while_loop])


# if __name__ == "__main__":
#     prog = cb.Builder()
#     default_post_op(prog, 2, 2, 32)
#     prog.program.emit()
