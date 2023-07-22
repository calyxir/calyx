# pylint: disable=import-error
import calyx.builder as cb


def insert_adder(
    comp: cb.ComponentBuilder,
    cell,
    port_l,
    port_r,
    ans_reg,
):
    """Inserts wiring into component {comp} to compute {port_l} + {port_r} and
      store it in {ans_reg}.

    1. Within component {comp}, creates a group called {cell}_group.
    2. Within {group}, create a {cell} that computes sums.
    3. Puts the values of {port_l} and {port_r} into {cell}.
    4. Then puts the answer of the computation into {ans_reg}.
    4. Returns the summing group.
    """
    adder = comp.add(cell, 32)
    with comp.group(f"{cell}_group") as adder_group:
        adder.left = port_l
        adder.right = port_r
        ans_reg.write_en = 1
        ans_reg.in_ = adder.out
        adder_group.done = ans_reg.done
    return adder_group


def insert_eq(comp: cb.ComponentBuilder, left, right, cell, width):
    """Inserts wiring into component {comp} to check if {a} == {b}.
    1. Within {comp}, creates a combinational group called {cell}_group.
    2. Within the group, creates a {cell} that checks equalities of {width}.
    3. Puts the values {a} and {b} into {cell}.
    4. Returns the equality-checking cell and the overall group.
    """
    eq_cell = comp.eq(cell, width)
    with comp.comb_group(f"{cell}_group") as eq_group:
        eq_cell.left = left
        eq_cell.right = right
    return eq_cell, eq_group


def insert_lt(comp: cb.ComponentBuilder, left, right, cell, width):
    """Inserts wiring into component {comp} to check if {a} < {b}.
    1. Within {comp}, creates a combinational group called {cell}_group.
    2. Within the group, creates a {cell} that checks less-than of {width}.
    3. Puts the values {a} and {b} into {cell}.
    4. Returns the less-than-checking cell and the overall group.
    """
    lt_cell = comp.lt(cell, width)
    with comp.comb_group(f"{cell}_group") as lt_group:
        lt_cell.left = left
        lt_cell.right = right
    return lt_cell, lt_group


def insert_add(comp: cb.ComponentBuilder, left, right, cell, width):
    """Inserts wiring into component {comp} to compute {a} + {b}.
    1. Within {comp}, creates a combinational group called {cell}_group.
    2. Within the group, creates a {cell} that computes sums of {width}.
    3. Puts the values {a} and {b} into {cell}.
    4. Returns the summing cell and the overall group.
    """
    add_cell = comp.add(cell, width)
    with comp.comb_group(f"{cell}_group") as add_group:
        add_cell.left = left
        add_cell.right = right
    return add_cell, add_group


def insert_sub(comp: cb.ComponentBuilder, left, right, cell, width):
    """Inserts wiring into component {comp} to compute {a} - {b}.
    1. Within {comp}, creates a combinational group called {cell}_group.
    2. Within the group, creates a {cell} that computes differences of {width}.
    3. Puts the values {a} and {b} into {cell}.
    4. Returns the subtracting cell and the overall group.
    """
    sub_cell = comp.sub(cell, width)
    with comp.comb_group(f"{cell}_group") as sub_group:
        sub_cell.left = left
        sub_cell.right = right
    return sub_cell, sub_group


def insert_incr(comp: cb.ComponentBuilder, reg, cell, group):
    """Inserts wiring into component {comp} to increment {reg} by 1.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, adds a cell {cell} that computes sums.
    3. Puts the values of {port} and 1 into {cell}.
    4. Then puts the answer of the computation back into {port}.
    4. Returns the group that does this.
    """
    incr_cell = comp.add(cell, 32)
    with comp.group(group) as incr_group:
        incr_cell.left = reg.out
        incr_cell.right = 1
        reg.write_en = 1
        reg.in_ = incr_cell.out
        incr_group.done = reg.done
    return incr_group


def insert_decr(comp: cb.ComponentBuilder, reg, cell, group):
    """Inserts wiring into component {comp} to decrement {reg} by 1.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, adds a cell {cell} that computes differences.
    3. Puts the values of {port} and 1 into {cell}.
    4. Then puts the answer of the computation back into {port}.
    4. Returns the group that does this.
    """
    decr_cell = comp.sub(cell, 32)
    with comp.group(group) as decr_group:
        decr_cell.left = reg.out
        decr_cell.right = cb.const(32, 1)
        reg.write_en = 1
        reg.in_ = decr_cell.out
        decr_group.done = reg.done
    return decr_group


def insert_reg_store(comp: cb.ComponentBuilder, reg, val, group):
    """Stores a value in a register.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, sets the register {reg} to {val}.
    3. Returns the group that does this.
    """
    with comp.group(group) as reg_grp:
        reg.in_ = val
        reg.write_en = 1
        reg_grp.done = reg.done
    return reg_grp


def insert_mem_load(comp: cb.ComponentBuilder, mem, i, reg, group):
    """Loads a value from one memory into a register.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from memory {mem} at address {i}.
    3. Writes the value into register {reg}.
    4. Returns the group that does this.
    """
    with comp.group(group) as load_grp:
        mem.addr0 = i
        reg.write_en = 1
        reg.in_ = mem.read_data
        load_grp.done = reg.done
    return load_grp


def insert_mem_load_to_mem(comp: cb.ComponentBuilder, mem, i, ans, j, group):
    """Loads a value from one memory into another.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from memory {mem} at address {i}.
    3. Writes the value into memory {ans} at address {j}.
    4. Returns the group that does this.
    """
    with comp.group(group) as load_grp:
        mem.addr0 = i
        ans.write_en = 1
        ans.addr0 = j
        ans.write_data = mem.read_data
        load_grp.done = ans.done
    return load_grp


def insert_sub_and_store(
    comp: cb.ComponentBuilder,
    port,
    const,
    cell,
    width,
    ans_reg,
):
    """Adds wiring into component {comp} to compute {port} - {const}
    and store it in {ans_reg}.
    1. Within component {comp}, creates a group called {cell}_group.
    2. Within {group}, create a {cell} that computes differences.
    3. Puts the values of {port} and {const} into {cell}.
    4. Then puts the answer of the computation into {ans_reg}.
    4. Returns the sub-checking group.
    """
    sub_cell = comp.sub(cell, width)
    with comp.group(f"{cell}_group") as sub_group:
        sub_cell.left = port
        sub_cell.right = const
        ans_reg.write_en = 1
        ans_reg.in_ = sub_cell.out
        sub_group.done = ans_reg.done
    return sub_group
