#!/usr/bin/env python3

import numpy as np
from futil import ast
from futil.utils import bits_needed

# Global constant for the current bitwidth.
BITWIDTH = 32
# Name of the ouput array
OUT_MEM = ast.CompVar("out_mem")
PE_NAME = "mac_pe"

# Eventually, PE_DEF will be included a separate `.futil` file.
PE_DEF = """
component mac_pe(top: 32, left: 32) -> (out: 32) {
  cells {
    // Storage
    acc = std_reg(32);
    mul_reg = std_reg(32);
    // Computation
    add = std_add(32);
    mul = std_mult_pipe(32);
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


def instantiate_indexor(prefix, width):
    """
    Instantiate an indexor for accessing memory with name `prefix`.
    Generates structure to initialize and update the indexor.

    The initializor starts sets the memories to their maximum value
    because we expect all indices to be incremented once before
    being used.

    Returns (cells, structure)
    """
    stdlib = ast.Stdlib()
    name = ast.CompVar(NAME_SCHEME["index name"].format(prefix=prefix))
    add_name = ast.CompVar(f"{prefix}_add")
    cells = [
        ast.Cell(name, stdlib.register(width)),
        ast.Cell(add_name, stdlib.op("add", width, signed=False)),
    ]

    init_name = ast.CompVar(NAME_SCHEME["index init"].format(prefix=prefix))
    init_group = ast.Group(
        init_name,
        connections=[
            ast.Connect(
                ast.ConstantPort(width, 2 ** width - 1), ast.CompPort(name, "in")
            ),
            ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(name, "write_en")),
            ast.Connect(ast.CompPort(name, "done"), ast.HolePort(init_name, "done")),
        ],
    )

    upd_name = ast.CompVar(NAME_SCHEME["index update"].format(prefix=prefix))
    upd_group = ast.Group(
        upd_name,
        connections=[
            ast.Connect(ast.ConstantPort(width, 1), ast.CompPort(add_name, "left")),
            ast.Connect(ast.CompPort(name, "out"), ast.CompPort(add_name, "right")),
            ast.Connect(ast.CompPort(add_name, "out"), ast.CompPort(name, "in")),
            ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(name, "write_en")),
            ast.Connect(ast.CompPort(name, "done"), ast.HolePort(upd_name, "done")),
        ],
    )

    return (cells, [init_group, upd_group])


def instantiate_memory(top_or_left, idx, size):
    """
    Instantiates:
    - top memory
    - structure to move data from memory to read registers.

    Returns (cells, structure) tuple.
    """
    if top_or_left == "top":
        name = f"t{idx}"
        target_reg = f"top_0_{idx}"
    elif top_or_left == "left":
        name = f"l{idx}"
        target_reg = f"left_{idx}_0"
    else:
        raise f"Invalid top_or_left: {top_or_left}"

    var_name = ast.CompVar(f"{name}")
    idx_name = ast.CompVar(NAME_SCHEME["index name"].format(prefix=name))
    group_name = ast.CompVar(NAME_SCHEME["memory move"].format(prefix=name))
    target_reg = ast.CompVar(target_reg)
    structure = ast.Group(
        group_name,
        connections=[
            ast.Connect(ast.CompPort(idx_name, "out"), ast.CompPort(var_name, "addr0")),
            ast.Connect(
                ast.CompPort(var_name, "read_data"), ast.CompPort(target_reg, "in")
            ),
            ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(target_reg, "write_en")),
            ast.Connect(
                ast.CompPort(target_reg, "done"), ast.HolePort(group_name, "done")
            ),
        ],
    )

    idx_width = bits_needed(size)
    # Instantiate the indexor
    (idx_cells, idx_structure) = instantiate_indexor(name, idx_width)
    idx_structure.append(structure)
    # Instantiate the memory
    idx_cells.append(
        ast.Cell(
            var_name, ast.Stdlib().mem_d1(BITWIDTH, size, idx_width), is_external=True
        )
    )
    return (idx_cells, idx_structure)


def instantiate_pe(row, col, right_edge=False, down_edge=False):
    """
    Instantiate the PE and all the registers connected to it.
    """
    # Add all the required cells.
    stdlib = ast.Stdlib()
    cells = [
        ast.Cell(ast.CompVar(f"pe_{row}_{col}"), ast.CompInst(PE_NAME, [])),
        ast.Cell(ast.CompVar(f"top_{row}_{col}"), stdlib.register(BITWIDTH)),
        ast.Cell(ast.CompVar(f"left_{row}_{col}"), stdlib.register(BITWIDTH)),
    ]
    return cells


def instantiate_data_move(row, col, right_edge, down_edge):
    """
    Generates groups for "data movers" which are groups that move data
    from the `write` register of the PE at (row, col) to the read register
    of the PEs at (row+1, col) and (row, col+1)
    """
    name = f"pe_{row}_{col}"
    structures = []

    if not right_edge:
        group_name = ast.CompVar(NAME_SCHEME["register move right"].format(pe=name))
        src_reg = ast.CompVar(f"left_{row}_{col}")
        dst_reg = ast.CompVar(f"left_{row}_{col + 1}")
        mover = ast.Group(
            group_name,
            connections=[
                ast.Connect(ast.CompPort(src_reg, "out"), ast.CompPort(dst_reg, "in")),
                ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(dst_reg, "write_en")),
                ast.Connect(
                    ast.CompPort(dst_reg, "done"), ast.HolePort(group_name, "done")
                ),
            ],
        )
        structures.append(mover)

    if not down_edge:
        group_name = ast.CompVar(NAME_SCHEME["register move down"].format(pe=name))
        src_reg = ast.CompVar(f"top_{row}_{col}")
        dst_reg = ast.CompVar(f"top_{row + 1}_{col}")
        mover = ast.Group(
            group_name,
            connections=[
                ast.Connect(ast.CompPort(src_reg, "out"), ast.CompPort(dst_reg, "in")),
                ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(dst_reg, "write_en")),
                ast.Connect(
                    ast.CompPort(dst_reg, "done"), ast.HolePort(group_name, "done")
                ),
            ],
        )
        structures.append(mover)

    return structures


def instantiate_output_move(row, col, row_idx_bitwidth, col_idx_bitwidth):
    """
    Generates groups to move the final value from a PE into the output array.
    """
    group_name = ast.CompVar(NAME_SCHEME["out mem move"].format(pe=f"pe_{row}_{col}"))
    pe = ast.CompVar(f"pe_{row}_{col}")
    return ast.Group(
        group_name,
        connections=[
            ast.Connect(
                ast.ConstantPort(row_idx_bitwidth, row), ast.CompPort(OUT_MEM, "addr0")
            ),
            ast.Connect(
                ast.ConstantPort(col_idx_bitwidth, col), ast.CompPort(OUT_MEM, "addr1")
            ),
            ast.Connect(ast.CompPort(pe, "out"), ast.CompPort(OUT_MEM, "write_data")),
            ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(OUT_MEM, "write_en")),
            ast.Connect(
                ast.CompPort(OUT_MEM, "done"), ast.HolePort(group_name, "done")
            ),
        ],
    )


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
    out = np.zeros((left_length, top_length, top_depth), dtype="i")
    out[0][0] = np.arange(top_depth)

    # Fill the first col: Every column runs one "step" behind the column on
    # its left.
    for col in range(1, top_length):
        out[0][col] = out[0][col - 1] + 1

    # Fill the remaining rows. Similarly, all rows run one "step" behind the
    # row on their top.
    for row in range(1, left_length):
        out[row][0] = out[row - 1][0] + 1
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
        return NAME_SCHEME["memory move"].format(prefix=f"t{col}")
    else:
        return NAME_SCHEME["register move down"].format(pe=f"pe_{row - 1}_{col}")


def col_data_mover_at(row, col):
    """
    Returns the name of the group that is abstractly at the location (row, col)
    in the "col data mover" matrix.
    """
    if col == 0:
        return NAME_SCHEME["memory move"].format(prefix=f"l{row}")
    else:
        return NAME_SCHEME["register move right"].format(pe=f"pe_{row}_{col - 1}")


def index_update_at(row, col):
    """
    Returns the name of the group that is abstractly at the location (row, col)
    in the "col data mover" matrix.
    """
    updates = []
    if row == 0:
        updates.append(NAME_SCHEME["index update"].format(prefix=f"t{col}"))

    if col == 0:
        updates.append(NAME_SCHEME["index update"].format(prefix=f"l{row}"))

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
    sch = schedule_to_timesteps(
        generate_schedule(top_length, top_depth, left_length, left_depth)
    )

    control = []

    # Initialize all memories.
    init_indices = [
        ast.Enable(NAME_SCHEME["index init"].format(prefix=f"t{idx}"))
        for idx in range(top_length)
    ]
    init_indices.extend(
        [
            ast.Enable(NAME_SCHEME["index init"].format(prefix=f"l{idx}"))
            for idx in range(left_length)
        ]
    )
    control.append(ast.ParComp(init_indices))

    # Increment memories for PE_00 before computing with it.
    upd_pe00_mem = []
    upd_pe00_mem.append(ast.Enable(NAME_SCHEME["index update"].format(prefix="t0")))
    upd_pe00_mem.append(ast.Enable(NAME_SCHEME["index update"].format(prefix="l0")))
    control.append(ast.ParComp(upd_pe00_mem))

    for (idx, elements) in enumerate(sch):
        # Move all the requisite data.
        move = [ast.Enable(row_data_mover_at(r, c)) for (r, c) in elements]
        move.extend([ast.Enable(col_data_mover_at(r, c)) for (r, c) in elements])
        control.append(ast.ParComp(move))

        # Update the indices if needed.
        more_control = []
        if idx < len(sch) - 1:
            next_elements = sch[idx + 1]
            upd_memory = [
                ast.Enable(upd)
                for (r, c) in next_elements
                if (r == 0 or c == 0)
                for upd in index_update_at(r, c)
            ]
            more_control.extend(upd_memory)

        # ast.Invoke the PEs and move the data to the next layer.
        for (r, c) in elements:
            more_control.append(
                ast.Invoke(
                    id=ast.CompVar(f"pe_{r}_{c}"),
                    args=[
                        ast.CompPort(ast.CompVar(f"top_{r}_{c}"), "out"),
                        ast.CompPort(ast.CompVar(f"left_{r}_{c}"), "out"),
                    ],
                    params=[ast.CompVar("top"), ast.CompVar("left")],
                )
            )

        control.append(ast.ParComp(more_control))

    # Move all the results into output memory
    mover_groups = []
    for row in range(left_length):
        for col in range(top_length):
            mover_groups.append(
                ast.Enable(NAME_SCHEME["out mem move"].format(pe=f"pe_{row}_{col}"))
            )

    control.append(ast.SeqComp(mover_groups))
    return ast.ControlEntry(ast.ControlEntryType.Seq, stmts=control)


def create_systolic_array(top_length, top_depth, left_length, left_depth):
    """
    top_length: Number of PEs in each row.
    top_depth: Number of elements processed by each PE in a row.
    left_length: Number of PEs in each column.
    left_depth: Number of elements processed by each PE in a col.
    """

    assert top_depth == left_depth, (
        f"Cannot multiply matrices: {top_length}x{top_depth} and "
        + "{left_depth}x{left_length}"
    )

    cells = []
    wires = []

    # Instantiate all the memories
    for r in range(top_length):
        (c, s) = instantiate_memory("top", r, top_depth)
        cells.extend(c)
        wires.extend(s)

    for c in range(left_length):
        (c, s) = instantiate_memory("left", c, left_depth)
        cells.extend(c)
        wires.extend(s)

    # Instantiate output memory
    out_ridx_size = bits_needed(left_length)
    out_cidx_size = bits_needed(top_length)
    cells.append(
        ast.Cell(
            OUT_MEM,
            ast.Stdlib().mem_d2(
                BITWIDTH, left_length, top_length, out_ridx_size, out_cidx_size
            ),
            is_external=True,
        )
    )

    # Instantiate all the PEs
    for row in range(left_length):
        for col in range(top_length):
            # Instantiate the PEs
            c = instantiate_pe(row, col, col == top_length - 1, row == left_length - 1)
            cells.extend(c)

            # Instantiate the mover fabric
            s = instantiate_data_move(
                row, col, col == top_length - 1, row == left_length - 1
            )
            wires.extend(s)

            # Instantiate output movement structure
            s = instantiate_output_move(row, col, out_ridx_size, out_cidx_size)
            wires.append(s)
    main = ast.Component(
        name="main",
        inputs=[],
        outputs=[],
        structs=wires + cells,
        controls=generate_control(top_length, top_depth, left_length, left_depth),
    )

    return ast.Program(imports=[ast.Import("primitives/std.lib")], components=[main])


if __name__ == "__main__":
    import argparse
    import json

    parser = argparse.ArgumentParser(description="Process some integers.")
    parser.add_argument("file", nargs="?", type=str)
    parser.add_argument("-tl", "--top-length", type=int)
    parser.add_argument("-td", "--top-depth", type=int)
    parser.add_argument("-ll", "--left-length", type=int)
    parser.add_argument("-ld", "--left-depth", type=int)

    args = parser.parse_args()

    top_length, top_depth, left_length, left_depth = None, None, None, None

    fields = [args.top_length, args.top_depth, args.left_length, args.left_depth]
    if all(map(lambda x: x is not None, fields)):
        top_length = args.top_length
        top_depth = args.top_depth
        left_length = args.left_length
        left_depth = args.left_depth
    elif args.file is not None:
        with open(args.file, "r") as f:
            spec = json.load(f)
            top_length = spec["top_length"]
            top_depth = spec["top_depth"]
            left_length = spec["left_length"]
            left_depth = spec["left_depth"]
    else:
        parser.error(
            "Need to pass either `-f FILE` or all of "
            + "`-tl TOP_LENGTH -td TOP_DEPTH -ll LEFT_LENGTH -ld LEFT_DEPTH`"
        )

    program = create_systolic_array(
        top_length=top_length,
        top_depth=top_depth,
        left_length=left_length,
        left_depth=left_depth,
    )
    program.emit()
    print(PE_DEF)
