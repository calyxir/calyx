# pylint: disable=import-error
import calyx.builder as cb


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


def mem_load_std_d1(comp: cb.ComponentBuilder, mem, i, reg, group):
    """Loads a value from one memory (std_d1) into a register.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from memory {mem} at address {i}.
    3. Writes the value into register {reg}.
    4. Returns the group that does this.
    """
    assert mem.is_std_mem_d1()
    with comp.group(group) as load_grp:
        mem.addr0 = i
        reg.write_en = 1
        reg.in_ = mem.read_data
        load_grp.done = reg.done
    return load_grp


def mem_store_std_d1(comp: cb.ComponentBuilder, mem, i, val, group):
    """Stores a value into a (std_d1) memory.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from {val}.
    3. Writes the value into memory {mem} at address i.
    4. Returns the group that does this.
    """
    assert mem.is_std_mem_d1()
    with comp.group(group) as store_grp:
        mem.addr0 = i
        mem.write_en = 1
        mem.write_data = val
        store_grp.done = mem.done
    return store_grp


def mem_read_seq_d1(comp: cb.ComponentBuilder, mem, i, group):
    """Given a seq_mem_d1, reads from memory at address i.
    Note that this does not write the value anywhere.

    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from memory {mem} at address {i},
       thereby "latching" the value.
    3. Returns the group that does this.
    """
    assert mem.is_seq_mem_d1()
    with comp.group(group) as read_grp:
        mem.addr0 = i
        mem.read_en = 1
        read_grp.done = mem.read_done
    return read_grp


def mem_write_seq_d1_to_reg(comp: cb.ComponentBuilder, mem, reg, group):
    """Given a seq_mem_d1 that is already assumed to have a latched value,
    reads the latched value and writes it to a register.

    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from memory {mem}.
    3. Writes the value into register {reg}.
    4. Returns the group that does this.
    """
    assert mem.is_seq_mem_d1()
    with comp.group(group) as write_grp:
        reg.write_en = 1
        reg.in_ = mem.read_data
        write_grp.done = reg.done
    return write_grp


def mem_store_seq_d1(comp: cb.ComponentBuilder, mem, i, val, group):
    """Given a seq_mem_d1, stores a value into memory at address i.

    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from {val}.
    3. Writes the value into memory {mem} at address i.
    4. Returns the group that does this.
    """
    assert mem.is_seq_mem_d1()
    with comp.group(group) as store_grp:
        mem.addr0 = i
        mem.write_en = 1
        mem.write_data = val
        store_grp.done = mem.write_done
    return store_grp


def insert_mem_load_to_mem(comp: cb.ComponentBuilder, mem, i, ans, j, group):
    """Loads a value from one std_mem_d1 memory into another.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from memory {mem} at address {i}.
    3. Writes the value into memory {ans} at address {j}.
    4. Returns the group that does this.
    """
    assert mem.is_std_mem_d1() and ans.is_std_mem_d1()
    with comp.group(group) as load_grp:
        mem.addr0 = i
        ans.write_en = 1
        ans.addr0 = j
        ans.write_data = mem.read_data
        load_grp.done = ans.done
    return load_grp


def insert_add_store_in_reg(
    comp: cb.ComponentBuilder,
    cellname,
    left,
    right,
    ans_reg=None,
):
    """Inserts wiring into component {comp} to compute {left} + {right} and
      store it in {ans_reg}.
    1. Within component {comp}, creates a group called {cellname}_group.
    2. Within {group}, create a cell {cellname} that computes sums.
    3. Puts the values of {left} and {right} into the cell.
    4. Then puts the answer of the computation into {ans_reg}.
    4. Returns the summing group and the register.
    """
    add_cell = comp.add(32, cellname)
    ans_reg = ans_reg or comp.reg(f"reg_{cellname}", 32)
    with comp.group(f"{cellname}_group") as adder_group:
        add_cell.left = left
        add_cell.right = right
        ans_reg.write_en = 1
        ans_reg.in_ = add_cell.out
        adder_group.done = ans_reg.done
    return adder_group, ans_reg


def insert_sub_store_in_reg(
    comp: cb.ComponentBuilder,
    left,
    right,
    cellname,
    width,
    ans_reg=None,
):
    """Adds wiring into component {comp} to compute {left} - {right}
    and store it in {ans_reg}.
    1. Within component {comp}, creates a group called {cellname}_group.
    2. Within {group}, create a cell {cellname} that computes differences.
    3. Puts the values of {left} and {right} into {cell}.
    4. Then puts the answer of the computation into {ans_reg}.
    4. Returns the subtracting group and the register.
    """
    sub_cell = comp.sub(width, cellname)
    ans_reg = ans_reg or comp.reg(f"reg_{cellname}", width)
    with comp.group(f"{cellname}_group") as sub_group:
        sub_cell.left = left
        sub_cell.right = right
        ans_reg.write_en = 1
        ans_reg.in_ = sub_cell.out
        sub_group.done = ans_reg.done
    return sub_group, ans_reg
