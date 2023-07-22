# pylint: disable=import-error
from typing import List
import calyx.builder as cb


def insert_adder(
    comp: cb.ComponentBuilder,
    adder: cb.CellBuilder,
    group,
    port_l,
    port_r,
    ans_reg,
):
    """To component {comp}, adds wiring for an group called {group}.
    Assumes the adder cell {adder} is in the component.
    In {group}, puts {port_l} and {port_r} into the {adder} cell.
    Then puts the output of {adder} into the memory register {ans_reg}.
    Returns the group.
    """
    with comp.group(group) as adder_group:
        adder.left = port_l
        adder.right = port_r
        ans_reg.write_en = 1
        ans_reg.in_ = adder.out
        adder_group.done = ans_reg.done
    return adder_group


def insert_eq(comp: cb.ComponentBuilder, port, const, cell, group):
    """Adds wiring into component {comp} to check if {port} == {const}.
    1. Within {comp}, creates a group called {group}.
    2. Within {group}, creates a cell called {cell} that checks equality.
    3. Puts the values of {port} and {const} into {cell}.
    4. Returns the equality-checking cell and the equality-checking group.
    """
    eq_cell = comp.eq(cell, 32)
    with comp.comb_group(group) as eq_group:
        eq_cell.left = port
        eq_cell.right = const
    return eq_cell, eq_group


def insert_lt(comp: cb.ComponentBuilder, port, const, cell, group):
    """Adds wiring into component {comp} to check if {port} < {const}.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, creates a cell called {cell} that checks for less-than.
    3. Puts the values of {port} and {const} into {cell}.
    4. Returns the less-than-checking cell and the less-than-checking group.
    """
    lt_cell = comp.lt(cell, 32)
    with comp.comb_group(group) as lt_group:
        lt_cell.left = port
        lt_cell.right = const
    return lt_cell, lt_group


def insert_sub(comp: cb.ComponentBuilder, port, const, sub_cell, ans_reg, group):
    """Adds wiring into component {comp} to compute {port} - {const}.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, assumes there is a cell {cell} that computes differences.
    3. Puts the values of {port} and {const} into {cell}.
    4. Then puts the answer of the computation into {ans_reg}.
    4. Returns the sub-checking group.
    """
    # Note, this one is a little different than the others.
    # 1. We assume the subtraction cell already exists.
    # 2. We're not returning the cell, because we don't need to.
    # 3. We write the answer into `ans_reg`.

    with comp.group(group) as sub_group:
        sub_cell.left = port
        sub_cell.right = const
        ans_reg.write_en = 1
        ans_reg.in_ = sub_cell.out
        sub_group.done = ans_reg.done
    return sub_group


def insert_mem_load(comp: cb.ComponentBuilder, mem, i, ans, group):
    """Loads a value from one memory into another.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from memory {mem} at address {i}.
    3. Writes the value into memory {ans} at address 0.
    4. Returns the group that does this.
    """
    with comp.group(group) as load_grp:
        mem.addr0 = i
        ans.write_en = 1
        ans.write_data = mem.read_data
        load_grp.done = ans.done
    return load_grp


def insert_reg_load(comp: cb.ComponentBuilder, port, ans_reg, group):
    """Creates a group called {group}.
    In that group, loads the value of {port} into {ans_reg}.
    Returns the group.
    """
    with comp.group(group) as grp:
        ans_reg.write_en = 1
        ans_reg.in_ = port
        grp.done = ans_reg.done
    return grp
