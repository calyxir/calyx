import textwrap
import math

# Global constant for the current bitwidth.
BITWIDTH = 32
PE_NAME = 'mac_pe'

# Naming scheme for generated groups. Used to keep group names consistent
# across structure and control.
NAME_SCHEME = {
    'index name': '{prefix}_idx',
    'index init': '{prefix}_idx_init',
    'index update': '{prefix}_idx_update',
    'memory move': '{prefix}_move'
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
        }}
    """

    upd_name = NAME_SCHEME['index update'].format(prefix=prefix)
    upd_group = f"""
        group {upd_name} {{
            {add_name}.left = {width}'d1;
            {add_name}.right = {name}.out;
            {name}.in = {add_name}.out;
            {name}.write_en = 1'd1;
            {upd_name}[done] = {name}.done;
        }}
    """

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
    group = f'{pe}_compute'
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


def pe_control(row, col):
    """
    Create control for the PE located at (row, col) in the array.
    """
    return ""


def generate_control(top_cols, left_rows):
    return ""


def create_systolic_array(top_length, top_depth, left_length, left_depth):
    """
    top_length: Number of PEs in each row.
    top_depth: Number of elements processed by each PE in a row.
    left_length: Number of PEs in each column.
    left_depth: Number of elements processed by each PE in a col.
    """

    cells = []
    wires = []
    control = []

    # Instantiate all the memories
    for r in range(top_length):
        (c, s) = instantiate_memory('top', r, top_depth)
        cells.append(c)
        wires.append(s)

    # Instantiate all the PEs
    for r in range(left_length):
        for c in range(top_length):
            (c, s) = instantiate_pe(
                r, c, r == left_length - 1, c == top_length - 1)
            cells.append(c)
            wires.append(s)

    cells_str = '\n'.join(cells)
    wires_str = '\n'.join(wires)
    control_str = '\n'.join(control)


    return textwrap.dedent(f"""
    import "primitives/std.lib";
    component {PE_NAME}(top: {BITWIDTH}, left: {BITWIDTH}) -> (down: {BITWIDTH}, right: {BITWIDTH}) {{
        cells {{}}
        wires {{}}
        control {{}}
    }}
    component main() -> () {{
        cells {{
            {textwrap.indent(cells_str, " "*10)}
        }}
        wires {{
            {textwrap.indent(wires_str, " "*10)}
        }}
        control {{
            {control_str}
        }}
    }}
    """)


if __name__ == '__main__':
    print(create_systolic_array(2, 2, 2, 2))
