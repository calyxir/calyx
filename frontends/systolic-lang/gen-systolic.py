#!/usr/bin/env python3

import sys
import textwrap
import math
import numpy as np
import argparse
import json

# Global constant for the current bitwidth.
BITWIDTH = 32
PE_NAME = 'mac_pe'
# Name of the ouput array
OUT_MEM = 'out_mem'
PE_DEF = """
component mac_pe(top: 32, left: 32) -> (out: 32) {
  cells {
    // Storage
    acc = prim std_reg(32);
    mul_reg = prim std_reg(32);
    // Computation
    add = prim std_add(32);
    mul = prim std_mult_pipe(32);
  }

  wires {

    group do_mul<"static"=4> {
      mul.left = top;
      mul.right = left;
      mul.go = !mul.done ? 1'd1;
      mul_reg.in = mul.done ? mul.out;
      mul_reg.write_en = mul.done ? 1'd1;
      do_mul[done] = mul_reg.done;
    }

    group do_add {
      add.left = acc.out;
      add.right = mul_reg.out;
      acc.in = add.out;
      acc.write_en = 1'd1;
      do_add[done] = acc.done;
    }

    out = acc.out;
  }

  control {
    seq { do_mul; do_add; }
  }
}"""

# Naming scheme for generated groups. Used to keep group names consistent
# across structure and control.
NAME_SCHEME = {
    # Indexing into the memory
    'index name': '{prefix}_idx',
    'index init': '{prefix}_idx_init',
    'index update': '{prefix}_idx_update',

    # Move data from main memories
    'memory move': '{prefix}_move',
    'out mem move': '{pe}_out_write',

    # Move data between internal registers
    'register move down': '{pe}_down_move',
    'register move right': '{pe}_right_move',
}


def bits_needed(num):
    """
    Number of bits needed to represent `num`.
    """
    return math.floor(math.log(num, 2)) + 1


def create_adder(name, width):
    return f'{name} = prim std_add({width});'


def create_register(name, width):
    return f'{name} = prim std_reg({width});'


def create_memory(name, bitwidth, size):
    """
    Defines a 1D memory.
    Returns (cells, idx_size)
    """
    idx_width = bits_needed(size)
    return (
        f'{name} = prim std_mem_d1({bitwidth}, {size}, {idx_width});',
        idx_width
    )


def instantiate_indexor(prefix, width):
    """
    Instantiate an indexor for accessing memory with name `prefix`.
    Generates structure to initialize and update the indexor.

    The initializor starts sets the memories to their maximum value
    because we expect all indices to be incremented once before
    being used.

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
        {name}.in = {width}'d{2**width-1};
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
        target_reg = f'top_0_{idx}'
    elif top_or_left == 'left':
        name = f'l{idx}'
        target_reg = f'left_{idx}_0'
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
    """
    # Add all the required cells.
    pe = f'pe_{row}_{col}'
    cells = [
        f'{pe} = {PE_NAME};',
        create_register(f'top_{row}_{col}', BITWIDTH),
        create_register(f'left_{row}_{col}', BITWIDTH),
    ]

    return '\n'.join(cells)


def instantiate_data_move(row, col, right_edge, down_edge):
    """
    Generates groups for "data movers" which are groups that move data
    from the `write` register of the PE at (row, col) to the read register
    of the PEs at (row+1, col) and (row, col+1)
    """
    name = f'pe_{row}_{col}'
    structures = []

    if not right_edge:
        group_name = NAME_SCHEME['register move right'].format(pe=name)
        src_reg = f'left_{row}_{col}'
        dst_reg = f'left_{row}_{col+1}'
        mover = textwrap.indent(textwrap.dedent(f'''
        group {group_name} {{
            {dst_reg}.in = {src_reg}.out;
            {dst_reg}.write_en = 1'd1;
            {group_name}[done] = {dst_reg}.done;
        }}'''), " " * 4)
        structures.append(mover)

    if not down_edge:
        group_name = NAME_SCHEME['register move down'].format(pe=name)
        src_reg = f'top_{row}_{col}'
        dst_reg = f'top_{row+1}_{col}'
        mover = textwrap.indent(textwrap.dedent(f'''
        group {group_name} {{
            {dst_reg}.in = {src_reg}.out;
            {dst_reg}.write_en = 1'd1;
            {group_name}[done] = {dst_reg}.done;
        }}'''), " "*4)
        structures.append(mover)

    return '\n'.join(structures)


def instantiate_output_move(row, col, row_idx_bitwidth, col_idx_bitwidth):
    """
    Generates groups to move the final value from a PE into the output array.
    """
    group_name = NAME_SCHEME['out mem move'].format(pe=f'pe_{row}_{col}')
    return textwrap.indent(textwrap.dedent(f'''
    group {group_name} {{
        {OUT_MEM}.addr0 = {row_idx_bitwidth}'d{row};
        {OUT_MEM}.addr1 = {col_idx_bitwidth}'d{col};
        {OUT_MEM}.write_data = pe_{row}_{col}.out;
        {OUT_MEM}.write_en = 1'd1;
        {group_name}[done] = {OUT_MEM}.done;
    }}'''), " " * 4)


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
        return NAME_SCHEME['register move down'].format(pe=f'pe_{row-1}_{col}')


def col_data_mover_at(row, col):
    """
    Returns the name of the group that is abstractly at the location (row, col)
    in the "col data mover" matrix.
    """
    if col == 0:
        return NAME_SCHEME['memory move'].format(prefix=f'l{row}')
    else:
        return NAME_SCHEME['register move right'].format(pe=f'pe_{row}_{col-1}')


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

    # print(sch)

    control = []

    # Initialize all memories.
    init_indices = [NAME_SCHEME['index init'].format(
        prefix=f't{idx}') for idx in range(top_length)]
    init_indices += [NAME_SCHEME['index init'].format(
        prefix=f'l{idx}') for idx in range(left_length)]

    nodes = ";\n        ".join(init_indices)
    control.append(f'''
    par {{
        {nodes};
    }}''')

    # Increment memories for PE_00 before computing with it.
    upd_pe00_mem = []
    upd_pe00_mem.append(NAME_SCHEME['index update'].format(prefix=f't0'))
    upd_pe00_mem.append(NAME_SCHEME['index update'].format(prefix=f'l0'))
    nodes = ";\n        ".join(upd_pe00_mem)
    control.append(f'''
    par {{
        {nodes};
    }}''')

    for (idx, elements) in enumerate(sch):
        # Move all the requisite data.
        move = [row_data_mover_at(r, c) for (r, c) in elements]
        move += [col_data_mover_at(r, c) for (r, c) in elements]
        nodes = ";\n            ".join(move)
        move_str = textwrap.indent(textwrap.dedent(f'''
        par {{
            {nodes};
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

        # Invoke the PEs and move the data to the next layer.
        for (r, c) in elements:
            more_control += [
                f'invoke pe_{r}_{c}(top = top_{r}_{c}.out, left = left_{r}_{c}.out)()',
            ]

        nodes = ";\n            ".join(more_control)
        more_control_str = textwrap.indent(textwrap.dedent(f'''
        par {{
            {nodes};
        }}'''), " " * 4)

        control.append(more_control_str)

    # Move all the results into output memory
    mover_groups = []
    for row in range(left_length):
        for col in range(top_length):
            g = NAME_SCHEME['out mem move'].format(pe=f'pe_{row}_{col}')
            mover_groups.append(g)

    nodes = ";\n        ".join(mover_groups)
    mover_control_str = textwrap.indent(textwrap.dedent(f'''
    seq {{
        {nodes};
    }}'''), " " * 4)
    control.append(mover_control_str)

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

    # Instantiate output memory
    out_ridx_size = bits_needed(left_length)
    out_cidx_size = bits_needed(top_length)
    o_mem = f'{OUT_MEM} = prim std_mem_d2_ext({BITWIDTH}, {left_length}, {top_length}, {out_ridx_size}, {out_cidx_size});'
    cells.append(o_mem)

    # Instantiate all the PEs
    for row in range(left_length):
        for col in range(top_length):
            # Instantiate the PEs
            c = instantiate_pe(
                row, col, col == top_length - 1, row == left_length - 1)
            cells.append(c)

            # Instantiate the mover fabric
            s = instantiate_data_move(
                row, col, col == top_length - 1, row == left_length - 1)
            wires.append(s)

            # Instantiate output movement structure
            s = instantiate_output_move(row, col, out_ridx_size, out_cidx_size)
            wires.append(s)

    cells_str = '\n'.join(cells)
    wires_str = '\n'.join(wires)
    control_str = generate_control(
        top_length, top_depth, left_length, left_depth)

    main = textwrap.dedent(f"""
    component main() -> () {{
        cells {{
{textwrap.indent(cells_str, " "*10)}
        }}
        wires {{
{textwrap.indent(wires_str, " "*6)}
        }}
        control {{
{textwrap.indent(control_str, " "*10)}
        }}
    }}
    """)

    return textwrap.dedent(f"""
import "primitives/std.lib";
    {PE_DEF}
    {main}
    """)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Process some integers.')
    parser.add_argument('file', nargs='?', type=str)
    parser.add_argument('-tl', '--top-length', type=int)
    parser.add_argument('-td', '--top-depth', type=int)
    parser.add_argument('-ll', '--left-length', type=int)
    parser.add_argument('-ld', '--left-depth', type=int)

    args = parser.parse_args()

    top_length, top_depth, left_length, left_depth = None, None, None, None

    fields = [args.top_length, args.top_depth, args.left_length, args.left_depth]
    if all(map(lambda x: x is not None, fields)):
        top_length = args.top_length
        top_depth = args.top_depth
        left_length = args.left_length
        left_depth = args.left_depth
    elif args.file is not None:
        with open(args.file, 'r') as f:
            spec = json.load(f)
            top_length = spec['top_length']
            top_depth = spec['top_depth']
            left_length = spec['left_length']
            left_depth= spec['left_depth']
    else:
        parser.error("Need to pass either `-f FILE` or all of `-tl TOP_LENGTH -td TOP_DEPTH -ll LEFT_LENGTH -ld LEFT_DEPTH`")

    out = create_systolic_array(
        top_length=top_length,
        top_depth=top_depth,
        left_length=left_length,
        left_depth=left_depth,
    )

    print(out)
