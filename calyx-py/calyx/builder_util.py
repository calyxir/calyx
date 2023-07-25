import calyx.builder as cb


def insert_eq(comp: cb.ComponentBuilder, a, b, cell, width):
    """Inserts wiring into component {comp} to check if {a} == {b}.
    1. Within {comp}, creates a combinational group called {cell}_group.
    2. Within the group, creates a {cell} that checks equalities of {width}.
    3. Puts the values {a} and {b} into {cell}.
    4. Returns the equality-checking cell and the overall group.
    """
    eq_cell = comp.eq(cell, width)
    with comp.comb_group(f"{cell}_group") as eq_group:
        eq_cell.left = a
        eq_cell.right = b
    return eq_cell, eq_group


def insert_neq(comp: cb.ComponentBuilder, a, b, cell, width):
    """Inserts wiring into component {comp} to check if {a} != {b}.
    1. Within {comp}, creates a combinational group called {cell}_group.
    2. Within the group, creates a {cell} that checks inequalities of {width}.
    3. Puts the values {a} and {b} into {cell}.
    4. Returns the inequality-checking cell and the overall group.
    """
    neq_cell = comp.neq(cell, width)
    with comp.comb_group(f"{cell}_group") as neq_group:
        neq_cell.left = a
        neq_cell.right = b
    return neq_cell, neq_group


def insert_incr(comp: cb.ComponentBuilder, reg, cell):
    """Inserts wiring into component {comp} to increment {reg} by 1.
    1. Within component {comp}, creates a group called cell_{group}.
    2. Within {group}, adds a cell {cell} that computes sums.
    3. Puts the values of {port} and 1 into {cell}.
    4. Then puts the answer of the computation back into {port}.
    4. Returns the group that does this.
    """
    incr_cell = comp.add(cell, 32)
    with comp.group(f"{cell}group") as incr_group:
        incr_cell.left = reg.out
        incr_cell.right = cb.const(32, 1)
        reg.write_en = 1
        reg.in_ = incr_cell.out
        incr_group.done = reg.done
    return incr_group


def insert_decr(comp: cb.ComponentBuilder, reg, cell):
    """Inserts wiring into component {comp} to decrement {reg} by 1.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, adds a cell {cell} that computes differences.
    3. Puts the values of {port} and 1 into {cell}.
    4. Then puts the answer of the computation back into {port}.
    4. Returns the group that does this.
    """
    decr_cell = comp.sub(cell, 32)
    with comp.group(f"{cell}group") as decr_group:
        decr_cell.left = reg.out
        decr_cell.right = cb.const(32, 1)
        reg.write_en = 1
        reg.in_ = decr_cell.out
        decr_group.done = reg.done
    return decr_group


def mem_load(comp: cb.ComponentBuilder, mem, i, reg, group):
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


def mem_store(comp: cb.ComponentBuilder, mem, i, val, group):
    """Stores a value from one memory into another.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from {val}.
    3. Writes the value into memory {mem} at address i.
    4. Returns the group that does this.
    """
    with comp.group(group) as store_grp:
        mem.addr0 = i
        mem.write_en = 1
        mem.write_data = val
        store_grp.done = mem.done
    return store_grp


def reg_store(comp: cb.ComponentBuilder, reg, val, group):
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


def mem_read_seqd1(comp: cb.ComponentBuilder, mem, i, group):
    """Given a seq_mem_d1, reads from memory at address i.
    Note that this does not write the value anywhere.
    """
    assert mem.is_seq_mem_d1
    with comp.group(group) as read_grp:
        mem.addr0 = i
        mem.read_en = 1
        read_grp.done = mem.read_done
    return read_grp


def mem_write_seqd1_to_reg(comp: cb.ComponentBuilder, mem, reg, group):
    """Given a seq_mem_d1 that is already assumed to have a latched value,
    reads the latched value and writes it to a register.
    """
    assert mem.is_seq_mem_d1
    with comp.group(group) as write_grp:
        reg.write_en = 1
        reg.in_ = mem.read_data
        write_grp.done = reg.done
    return write_grp


def mem_store_seq_d1(comp: cb.ComponentBuilder, mem, i, val, group):
    """Stores a value from one memory into another.
    1. Within component {comp}, creates a group called {group}.
    2. Within {group}, reads from {val}.
    3. Writes the value into memory {mem} at address i.
    4. Returns the group that does this.
    """
    assert mem.is_seq_mem_d1
    with comp.group(group) as store_grp:
        mem.addr0 = i
        mem.write_en = 1
        mem.write_data = val
        store_grp.done = mem.write_done
    return store_grp


def reg_swap(comp: cb.ComponentBuilder, a, b, group):
    """Swaps the values of two registers.
    1. Within component {comp}, creates a group called {group}.
    2. Reads the value of {a} into a temporary register.
    3. Writes the value of {b} into {a}.
    4. Writes the value of the temporary register into {b}.
    5. Returns the group that does this.
    """
    with comp.group(group) as swap_grp:
        tmp = comp.reg("tmp", 1)
        tmp.write_en = 1
        tmp.in_ = a.out
        a.write_en = 1
        a.in_ = b.out
        b.write_en = 1
        b.in_ = tmp.out
        swap_grp.done = b.done
    return swap_grp
