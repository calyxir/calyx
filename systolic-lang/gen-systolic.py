import textwrap

# Global constant for the current bitwidth.
BITWIDTH = 32
PE_NAME = 'mac_pe'


def create_register(name, width):
    return f'{name} = prim std_reg({width});'


def create_memory(name, bitwidth, size):
    idx_width = math.floor(math.log(size, 2)) + 1
    return f'{name} = prim std_mem_d1({bitwidth}, {size}, {idx_width});'

def instantiate_top_memory(idx):
    """
    Instantiate a top memory, its indexor, the indexors update structure,
    and structure to move data from memory to read registers.
    """
    return ""

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
        create_register(f'lef_{row}{col}_read', BITWIDTH),
    ]
    if not right_edge:
        cells.append(create_register(f'right_{row}{col}_write', BITWIDTH))
    if not down_edge:
        cells.append(create_register(f'down_{row}{col}_write', BITWIDTH))

    structure_stmts = f"""
            {pe}.go = !{pe}.done ? 1'd1;
            {pe}.top = top_{row}{col}_read;
            {pe}.left = left_{row}{col}_read;"""

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
