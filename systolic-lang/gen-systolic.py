import textwrap
import math
import numpy as np

# Global constant for the current bitwidth.
BITWIDTH = 32
PE_NAME = 'mac_pe'
PE_DEF = """
component mac_pe(top: 32, left: 32) -> (down: 32, right: 32) {
  cells {
    // Storage
    acc = prim std_reg(32);
    mul_reg = prim std_reg(32);
    // Computation
    add = prim std_add(32);
    mul = prim std_mult(32);
  }

  wires {

    group do_mul {
      mul.left = top;
      mul.right = left;
      mul_reg.in = mul.out;
      mul_reg.write_en = 1'd1;
      do_mul[done] = mul_reg.done;
    }

    group do_add {
      add.left = acc.out;
      add.right = mul_reg.out;
      acc.in = add.out;
      acc.write_en = 1'd1;
      do_add[done] = acc.done;
    }

    down = top;
    right = left;
  }

  control {
    seq { do_mul; do_add; }
  }
}"""

# Naming scheme for generated groups. Used to keep group names consistent
# across structure and control.
NAME_SCHEME = {
    'index name': '{prefix}_idx',
    'index init': '{prefix}_idx_init',
    'index update': '{prefix}_idx_update',
    'memory move': '{prefix}_move',
    'pe compute': 'pe_{row}{col}_compute',
    'register move down': '{pe}_down_move',
    'register move right': '{pe}_right_move',
}


def create_adder(name, width):
    return f'{name} = prim std_add({width});'


def create_register(name, width):
    return f'{name} = prim std_reg({width});'


def create_memory(name, bitwidth, size):
    """
    Defines a 1D memory.
    Returns (cells, idx_size)
    """
    idx_width = math.floor(math.log(size, 2)) + 1
    return (
        f'{name} = prim std_mem_d1({bitwidth}, {size}, {idx_width});',
        idx_width
    )


def instantiate_indexor(prefix, width):
    """
    Instantiate an indexor for accessing memory with name `prefix`.
    Generates structure to initialize and update the indexor.
    Returns (cells, structure)
    """
    name = NAME_SCHEME['index name'].format(prefix=prefix)
    add_name = f'{prefix}_add'
    cells = [
        create_register(name, width),
        create_adder(add_name, width),
    ]

    init_name = NAME_SCHEME['index init'].format(prefix=prefix)
    init_group = f"""
    group {init_name} {{
        {name}.in = {width}'d0;
        {name}.write_en = 1'd1;
        {init_name}[done] = {name}.done;
    }}"""

    upd_name = NAME_SCHEME['index update'].format(prefix=prefix)
    upd_group = f"""
    group {upd_name} {{
        {add_name}.left = {width}'d1;
        {add_name}.right = {name}.out;
        {name}.in = {add_name}.out;
        {name}.write_en = 1'd1;
        {upd_name}[done] = {name}.done;
    }}"""

    return (
        '\n'.join(cells),
        (init_group + '\n' + upd_group)
    )


def instantiate_memory(top_or_left, idx, size):
    """
    Instantiates:
    - top memory
    - structure to move data from memory to read registers.

    Returns (cells, structure) tuple.
    """
    if top_or_left == 'top':
        name = f't{idx}'
        target_reg = f'top_0{idx}_read'
    elif top_or_left == 'left':
        name = f'l{idx}'
        target_reg = f'left_{idx}0_read'
    else:
        raise f'Invalid top_or_left: {top_or_left}'

    name = f'{name}'
    idx_name = NAME_SCHEME["index name"].format(prefix=name)
    group_name = NAME_SCHEME['memory move'].format(prefix=name)
    structure = f"""
    group {group_name} {{
        {name}.addr0 = {idx_name}.out;
        {target_reg}.in = {name}.read_data;
        {target_reg}.write_en = 1'd1;
        {group_name}[done] = {target_reg}.done;
    }}"""

    # Instantiate the memory
    (cell, idx_width) = create_memory(f'{name}', BITWIDTH, size)
    # Instantiate the indexor
    (idx_cells, idx_structure) = instantiate_indexor(name, idx_width)
    return (
        idx_cells + '\n' + cell,
        idx_structure + '\n' + structure
    )


def instantiate_pe(row, col, right_edge=False, down_edge=False):
    """
    Instantiate the PE and all the registers connected to it.
    Returns (cells, structure) tuple.
    """
    # Add all the required cells.
    pe = f'pe_{row}{col}'
    group = NAME_SCHEME['pe compute'].format(row=row, col=col)
    cells = [
        f'{pe} = {PE_NAME};',
        create_register(f'top_{row}{col}_read', BITWIDTH),
        create_register(f'left_{row}{col}_read', BITWIDTH),
    ]
    if not right_edge:
        cells.append(create_register(f'right_{row}{col}_write', BITWIDTH))
    if not down_edge:
        cells.append(create_register(f'down_{row}{col}_write', BITWIDTH))

    structure_stmts = f"""
            {pe}.go = !{pe}.done ? 1'd1;
            {pe}.top = top_{row}{col}_read.out;
            {pe}.left = left_{row}{col}_read.out;"""

    # Ports guarding the done condition for this group.
    done_guards = []

    if not right_edge:
        done_guards.append(f"right_{row}{col}_write.done")
        structure_stmts += f"""

            right_{row}{col}_write.in = {pe}.done ? {pe}.right;
            right_{row}{col}_write.write_en = {pe}.done ? 1'd1;"""

    if not down_edge:
        done_guards.append(f"top_{row}{col}_write.done")
        structure_stmts += f"""

            down_{row}{col}_write.in = {pe}.done ? {pe}.down;
            down_{row}{col}_write.write_en = {pe}.done ? 1'd1;"""

    # Special case: If there is no write register guard, guard using the
    # the PE.
    if len(done_guards) == 0:
        done_guards.append(f"{pe}.done")

    # Add the done condition for this group.
    guard = ' & '.join(done_guards)
    structure_stmts += f"""

            {group}[done] = {guard} ? 1'd1;"""

    structure = f"""
    group {group} {{
        {textwrap.indent(textwrap.dedent(structure_stmts), 6*" ")}
    }}"""

    return ('\n'.join(cells), textwrap.dedent(structure))


def instantiate_data_move(row, col, right_edge, down_edge):
    """
    Generates groups for "data movers" which are groups that move data
    from the `write` register of the PE at (row, col) to the read register
    of the PEs at (row+1, col) and (row, col+1)
    """
    name = f'pe_{row}{col}'
    structures = []

    if not right_edge:
        group_name = NAME_SCHEME['register move right'].format(pe=name)
        src_reg = f'right_{row}{col}_write'
        dst_reg = f'left_{row}{col+1}_read'
        mover = textwrap.indent(textwrap.dedent(f'''
        group {group_name} {{
            {dst_reg}.in = {src_reg}.out;
            {dst_reg}.write_en = 1'd1;
            {group_name}[done] = {dst_reg}.done;
        }}'''), " " * 4)
        structures.append(mover)

    if not down_edge:
        group_name = NAME_SCHEME['register move down'].format(pe=name)
        src_reg = f'down_{row}{col}_write'
        dst_reg = f'top_{row+1}{col}_read'
        mover = textwrap.indent(textwrap.dedent(f'''
        group {group_name} {{
            {dst_reg}.in = {src_reg}.out;
            {dst_reg}.write_en = 1'd1;
            {group_name}[done] = {dst_reg}.done;
        }}'''), " "*4)
        structures.append(mover)

    return '\n'.join(structures)


def generate_schedule(top_length, top_depth, left_length, left_depth):
    """
    Generate the *schedule* for each PE and data mover. A schedule is the
    timesteps when a PE needs to compute.

    Returns a schedule array `sch` of size `top_length`x`left_length` array
    such that `sch[i][j]` returns the timesteps when PE_{i}{j} is active.

    Timesteps start from 0.

    The schedule for matrix multiply looks like:

    (0, 1,..top_depth)  ->  (1, 2,..top_depth + 1)
            |
            V
    (1, 2,..top_depth + 1) -> ...
    """
    # The process of calculating the schedule starts from the leftmost
    # topmost element which is active from 0..top_depth timesteps.
    out = np.zeros((left_length, top_length, top_depth), dtype='i')
    out[0][0] = np.arange(top_depth)

    # Fill the first col: Every column runs one "step" behind the column on
    # its left.
    for col in range(1, top_length):
        out[0][col] = out[0][col - 1] + 1

    # Fill the remaining rows. Similarly, all rows run one "step" behind the
    # row on their top.
    for row in range(1, left_length):
        out[row][0] = out[row-1][0] + 1
        for col in range(1, top_length):
            out[row][col] = out[row][col - 1] + 1

    return out


def schedule_to_timesteps(schedule):
    """
    Transforms a *schedule*, which is a two dimensional array that contains
    a list of timesteps when the corresponding element is active, into
    an array with all the elements active at that index.

    Returns an array A s.t. A[i] returns the list of all elements active
    in time step `i`.
    """
    max_timestep = np.max(schedule.flatten())
    out = [[] for _ in range(max_timestep + 1)]
    (rows, cols, _) = schedule.shape

    for row in range(rows):
        for col in range(cols):
            for time_step in schedule[row][col]:
                out[time_step].append((row, col))

    return out


def row_data_mover_at(row, col):
    """
    Returns the name of the group that is abstractly at the location (row, col)
    in the "row data mover" matrix.
    """
    if row == 0:
        return NAME_SCHEME['memory move'].format(prefix=f't{col}')
    else:
        return NAME_SCHEME['register move down'].format(pe=f'pe_{row-1}{col}')


def col_data_mover_at(row, col):
    """
    Returns the name of the group that is abstractly at the location (row, col)
    in the "col data mover" matrix.
    """
    if col == 0:
        return NAME_SCHEME['memory move'].format(prefix=f'l{row}')
    else:
        return NAME_SCHEME['register move right'].format(pe=f'pe_{row}{col-1}')


def index_update_at(row, col):
    """
    Returns the name of the group that is abstractly at the location (row, col)
    in the "col data mover" matrix.
    """
    updates = []
    if row == 0:
        updates.append(NAME_SCHEME['index update'].format(prefix=f't{col}'))

    if col == 0:
        updates.append(NAME_SCHEME['index update'].format(prefix=f'l{row}'))

    return updates


def generate_control(top_length, top_depth, left_length, left_depth):
    """
    Logically, control performs the following actions:
    1. Initialize all the memory indexors at the start.
    2. For each time step in the schedule:
        a. Move the data required by PEs in this cycle.
        b. Update the memory indices if needed.
        c. Run the PEs that need to be active this cycle.
    """
    sch = schedule_to_timesteps(generate_schedule(
        top_length, top_depth, left_length, left_depth))

    control = []

    # Initialize all memories.
    init_indices = [NAME_SCHEME['index init'].format(
        prefix=f't{idx}') for idx in range(top_length)]
    init_indices += [NAME_SCHEME['index init'].format(
        prefix=f'l{idx}') for idx in range(left_length)]
    control.append(f'''
    par {{
        {"; ".join(init_indices)};
    }}''')

    for (idx, elements) in enumerate(sch):
        # Move all the requisite data.
        move = [row_data_mover_at(r, c) for (r, c) in elements]
        move += [col_data_mover_at(r, c) for (r, c) in elements]
        move_str = textwrap.indent(textwrap.dedent(f'''
        par {{
            {"; ".join(move)};
        }}'''), " " * 4)
        control.append(move_str)

        # Update the indices if needed.
        more_control = []
        if idx < len(sch) - 1:
            next_elements = sch[idx+1]
            upd_memory = [
                upd
                for (r, c) in next_elements if (r == 0 or c == 0)
                for upd in index_update_at(r, c)
            ]
            more_control += upd_memory
        # Enable the PEs
        more_control += [
            NAME_SCHEME['pe compute'].format(row=r, col=c) for (r, c) in elements
        ]

        more_control_str = textwrap.indent(textwrap.dedent(f'''
        par {{
            {"; ". join(more_control)};
        }}'''), " " * 4)

        control.append(more_control_str)

    all_control = "".join(control)
    return textwrap.dedent(f'''
    seq {{
        {textwrap.indent(all_control, " "*2)}
    }}''')


def create_systolic_array(top_length, top_depth, left_length, left_depth):
    """
    top_length: Number of PEs in each row.
    top_depth: Number of elements processed by each PE in a row.
    left_length: Number of PEs in each column.
    left_depth: Number of elements processed by each PE in a col.
    """

    assert top_depth == left_depth, f'Cannot multiply matrices: {top_length}x{top_depth} and {left_depth}x{left_length}'

    cells = []
    wires = []

    # Instantiate all the memories
    for r in range(top_length):
        (c, s) = instantiate_memory('top', r, top_depth)
        cells.append(c)
        wires.append(s)

    for c in range(left_length):
        (c, s) = instantiate_memory('left', c, left_depth)
        cells.append(c)
        wires.append(s)

    # Instantiate all the PEs
    for row in range(left_length):
        for col in range(top_length):
            # Instantiate the PEs
            (c, s) = instantiate_pe(
                row, col, col == top_length - 1, row == left_length - 1)
            cells.append(c)
            wires.append(s)

            # Instantiate the mover fabric
            s = instantiate_data_move(
                row, col, col == top_length - 1, row == left_length - 1)
            wires.append(s)

    cells_str = '\n'.join(cells)
    wires_str = '\n'.join(wires)
    control_str = generate_control(
        top_length, top_depth, left_length, left_depth)

    return textwrap.dedent(f"""
    import "primitives/std.lib";
    {PE_DEF}
    component main() -> () {{
        cells {{
            {textwrap.indent(cells_str, " "*10)}
        }}
        wires {{
            {textwrap.indent(wires_str, " "*10)}
        }}
        control {{
            {textwrap.indent(control_str, " "*10)}
        }}
    }}
    """)


if __name__ == '__main__':
    out = create_systolic_array(1, 4, 1, 4)
    print(out)
