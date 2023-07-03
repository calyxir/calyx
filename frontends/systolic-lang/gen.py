#!/usr/bin/env python3

import numpy as np
import calyx.builder as cb
from calyx import py_ast
from calyx.utils import bits_needed

# Global constant for the current bitwidth.
BITWIDTH = 32
# Name of the ouput array
OUT_MEM = "out_mem"
PE_NAME = "mac_pe"


def pe(prog: cb.Builder):
    comp = prog.component(PE_NAME)
    comp.input("top", BITWIDTH)
    comp.input("left", BITWIDTH)
    comp.output("out", BITWIDTH)
    acc = comp.reg("acc", BITWIDTH)
    add = comp.add("add", BITWIDTH)
    mul = comp.cell("mul", py_ast.Stdlib.op("mult_pipe", BITWIDTH, False))

    with comp.group("do_add") as do_add:
        add.left = acc.out
        add.right = mul.out
        acc.in_ = add.out
        acc.write_en = 1
        do_add.done = acc.done

    this = comp.this()
    with comp.continuous:
        this.out = acc.out

    comp.control += [
        py_ast.Invoke(
            py_ast.CompVar("mul"),
            [
                ("left", py_ast.ThisPort(py_ast.CompVar("top"))),
                ("right", py_ast.ThisPort(py_ast.CompVar("left"))),
            ],
            [],
        ),
        do_add,
    ]


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


def instantiate_indexor(comp: cb.ComponentBuilder, prefix, width) -> cb.CellBuilder:
    """
    Instantiate an indexor for accessing memory with name `prefix`.
    Generates structure to initialize and update the indexor.

    The initializor starts sets the memories to their maximum value
    because we expect all indices to be incremented once before
    being used.

    Returns (cells, structure)
    """
    name = NAME_SCHEME["index name"].format(prefix=prefix)

    reg = comp.reg(name, width)
    add = comp.add(f"{prefix}_add", width)

    init_name = NAME_SCHEME["index init"].format(prefix=prefix)
    with comp.group(init_name) as init:
        # Initialize the indexor to 0
        reg.in_ = 0
        reg.write_en = 1
        init.done = reg.done

    upd_name = NAME_SCHEME["index update"].format(prefix=prefix)
    with comp.group(upd_name) as upd:
        # Increment the indexor.
        add.left = 1
        add.right = reg.out
        reg.in_ = add.out
        reg.write_en = 1
        upd.done = reg.done

    return reg


def instantiate_memory(comp: cb.ComponentBuilder, top_or_left, idx, size):
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
        raise Exception(f"Invalid top_or_left: {top_or_left}")

    idx_width = bits_needed(size)
    # Instantiate the memory
    mem = comp.mem_d1(
        name,
        BITWIDTH,
        size,
        idx_width,
        is_external=True,
    )
    # Instantiate the indexing register
    idx = instantiate_indexor(comp, name, idx_width)
    # Register to save the value from the memory. Defined by [[instantiate_pe]].
    target = comp.get_cell(target_reg)
    group_name = NAME_SCHEME["memory move"].format(prefix=name)
    with comp.group(group_name) as move:
        mem.addr0 = idx.out
        target.in_ = mem.read_data
        target.write_en = 1
        move.done = target.done


def instantiate_pe(comp: cb.ComponentBuilder, row: int, col: int):
    """
    Instantiate the PE and all the registers connected to it.
    """
    # Add all the required cells.
    comp.cell(f"pe_{row}_{col}", py_ast.CompInst(PE_NAME, []))
    comp.reg(f"top_{row}_{col}", BITWIDTH)
    comp.reg(f"left_{row}_{col}", BITWIDTH)


def instantiate_data_move(
    comp: cb.ComponentBuilder, row: int, col: int, right_edge: bool, down_edge: bool
):
    """
    Generates groups for "data movers" which are groups that move data
    from the `write` register of the PE at (row, col) to the read register
    of the PEs at (row+1, col) and (row, col+1)
    """
    name = f"pe_{row}_{col}"

    if not right_edge:
        group_name = NAME_SCHEME["register move right"].format(pe=name)
        src_reg = comp.get_cell(f"left_{row}_{col}")
        dst_reg = comp.get_cell(f"left_{row}_{col + 1}")
        with comp.group(group_name) as move:
            dst_reg.in_ = src_reg.out
            dst_reg.write_en = 1
            move.done = dst_reg.done

    if not down_edge:
        group_name = NAME_SCHEME["register move down"].format(pe=name)
        src_reg = comp.get_cell(f"top_{row}_{col}")
        dst_reg = comp.get_cell(f"top_{row + 1}_{col}")
        with comp.group(group_name) as move:
            dst_reg.in_ = src_reg.out
            dst_reg.write_en = 1
            move.done = dst_reg.done


def instantiate_output_move(comp: cb.ComponentBuilder, row, col, cols):
    """
    Generates groups to move the final value from a PE into the output array.
    """
    group_name = NAME_SCHEME["out mem move"].format(pe=f"pe_{row}_{col}")
    idx = row * cols + col
    pe = comp.get_cell(f"pe_{row}_{col}")
    out = comp.get_cell(OUT_MEM)
    with comp.group(group_name) as move:
        out.addr0 = idx
        out.write_data = pe.out
        out.write_en = 1
        move.done = out.done


def gen_active_ranges(top_length, top_depth, left_length, left_depth):
    out = np.zeros((left_length, top_length), dtype="i")
    for row in range(0, left_length):
        for col in range(0, top_length):
            # PE at [row][col] is active for iterations [row_col, row + col + left_depth)
            # (could've chosen top_depth instead since we know left_depth == top_depth)
            out[row][col] = (row + col, row + col + left_depth)
    return out


def instantiate_while_groups(comp: cb.ComponentBuilder, width, limit):
    """
    comp: cb.ComponentBuilder, row, col, cols
    """
    reg = comp.reg("idx", width)
    add = comp.add(f"idx_add", width)
    lt = comp.lt(f"idx_lt_cell", width)
    with comp.group("incr_idx") as incr_grp:
        add.left = reg.out
        add.right = 1
        reg.in_ = add.out
        reg.write_en = 1
        incr_grp.done = reg.done
    with comp.comb_group("idx_lt_group") as idx_lt:
        lt.left = reg.out
        lt.right = limit


def instantiate_idx_between(comp: cb.ComponentBuilder, lo, hi, width) -> list:
    if lo == 0:
        lt = comp.lt(f"idx_between_{lo}_{hi}_cell", width)
        idx = comp.get_cell("idx")
        with comp.comb_group(f"idx_between_{lo}_{hi}_group") as idx_between:
            lt.left = idx.out
            lt.right = hi
    else:
        lt = comp.lt(f"index_lt_{hi}", width)
        ge = comp.ge(f"index_ge_{lo}", width)
        and_ = comp.and_(f"idx_between_{lo}_{hi}_cell", 1)
        idx = comp.get_cell("idx")
        with comp.comb_group(f"idx_between_{lo}_{hi}_group") as idx_between:
            ge.left = idx.out
            ge.right = lo
            lt.left = idx.out
            lt.right = hi
            and_.left = ge.out
            and_.right = lt.out


def get_movers(idx, top_length, left_length):
    # get movers active at [idx, idx + depth)
    movers = []
    if idx < left_length:
        movers.append(NAME_SCHEME["memory move"].format(prefix=f"l{idx}"))
    if idx < top_length:
        movers.append(NAME_SCHEME["memory move"].format(prefix=f"t{idx}"))
    for r in range(left_length):
        for c in range(top_length):
            if idx - 1 == r + c:
                if r < left_length - 1:
                    movers.append(
                        NAME_SCHEME["register move down"].format(pe=f"pe_{r}_{c}")
                    )
                if c < top_length - 1:
                    movers.append(
                        NAME_SCHEME["register move right"].format(pe=f"pe_{r}_{c}")
                    )
    mover_enables = [py_ast.Enable(name) for name in movers]
    return mover_enables


def get_pe_invokes(idx, top_length, left_length):
    """
    get PE invokes for [idx, idx + depth)
    """
    pe_invokes = []
    for x in range(left_length):
        for y in range(top_length):
            if idx == x + y:
                invoke = py_ast.Invoke(
                    id=py_ast.CompVar(f"pe_{x}_{y}"),
                    in_connects=[
                        ("top", py_ast.CompPort(py_ast.CompVar(f"top_{x}_{y}"), "out")),
                        (
                            "left",
                            py_ast.CompPort(py_ast.CompVar(f"left_{x}_{y}"), "out"),
                        ),
                    ],
                    out_connects=[],
                )
                pe_invokes.append(invoke)
    return pe_invokes


def get_idx_updates(idx, top_length, left_length):
    """
    get idx invokes for [idx, idx + depth)
    """
    idx_updates = []
    if idx < left_length:
        idx_updates.append(NAME_SCHEME["index update"].format(prefix=f"l{idx}"))
    if idx < top_length:
        idx_updates.append(NAME_SCHEME["index update"].format(prefix=f"t{idx}"))
    update_enables = [py_ast.Enable(name) for name in idx_updates]
    return update_enables


def generate_control(
    comp: cb.ComponentBuilder, top_length, top_depth, left_length, left_depth
):
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
    init_indices: list[py_ast.Control] = [
        py_ast.Enable(NAME_SCHEME["index init"].format(prefix=f"t{idx}"))
        for idx in range(top_length)
    ]
    init_indices.extend(
        [
            py_ast.Enable(NAME_SCHEME["index init"].format(prefix=f"l{idx}"))
            for idx in range(left_length)
        ]
    )
    control.append(py_ast.ParComp(init_indices))

    # source_pos metadata init
    init_tag = 0
    source_map = {}

    def counter():
        nonlocal init_tag
        old = init_tag
        init_tag += 1
        return old

    # end source pos init

    while_comb_group = comp.get_group("idx_lt_group")

    first_control_stmts = []
    second_control_stmts = []
    for idx in range(top_length + left_length - 1):
        hi = idx + top_depth
        if_cell = comp.get_cell(f"idx_between_{idx}_{hi}_cell")
        if_comb_group = comp.get_group(f"idx_between_{idx}_{hi}_group")

        # get movers
        movers = get_movers(idx, top_length, left_length)
        if_stmt = cb.if_(if_cell.out, if_comb_group, py_ast.ParComp(movers))
        first_control_stmts.append(if_stmt)

        # get second control
        pe_invokes = get_pe_invokes(idx, top_length, left_length)
        idx_updates = get_idx_updates(idx, top_length, left_length)
        if_stmt2 = cb.if_(
            if_cell.out, if_comb_group, py_ast.ParComp(pe_invokes + idx_updates)
        )
        second_control_stmts.append(if_stmt2)

    while_body = py_ast.SeqComp(
        [
            py_ast.ParComp(first_control_stmts),
            py_ast.ParComp(second_control_stmts),
            py_ast.Enable("incr_idx"),
        ]
    )
    while_cell = comp.get_cell("idx_lt_cell")
    while_comb_group = comp.get_group("idx_lt_group")
    while_loop = cb.while_(while_cell.out, while_comb_group, while_body)

    control.append(while_loop)

    # Move all the results into output memory
    mover_groups = []
    for row in range(left_length):
        for col in range(top_length):
            mover_groups.append(
                py_ast.Enable(NAME_SCHEME["out mem move"].format(pe=f"pe_{row}_{col}"))
            )

    control.append(py_ast.SeqComp(mover_groups))
    return py_ast.SeqComp(stmts=control), source_map


def create_systolic_array(
    prog: cb.Builder, top_length, top_depth, left_length, left_depth
):
    """
    top_length: Number of PEs in each row.
    top_depth: Number of elements processed by each PE in a row.
    left_length: Number of PEs in each column.
    left_depth: Number of elements processed by each PE in a col.
    """

    assert top_depth == left_depth, (
        f"Cannot multiply matrices: "
        f"{top_length}x{top_depth} and {left_depth}x{left_length}"
    )

    main = prog.component("main")

    for row in range(left_length):
        for col in range(top_length):
            # Instantiate the PEs and surronding registers
            instantiate_pe(main, row, col)

    # Instantiate all the memories
    for r in range(top_length):
        instantiate_memory(main, "top", r, top_depth)

    for col in range(left_length):
        instantiate_memory(main, "left", col, left_depth)

    # Instantiate output memory
    total_size = left_length * top_length
    out_idx_size = bits_needed(total_size)
    main.mem_d1(
        OUT_MEM,
        BITWIDTH,
        total_size,
        out_idx_size,
        is_external=True,
    )

    # Instantiate all the PEs
    for row in range(left_length):
        for col in range(top_length):
            # Instantiate the mover fabric
            instantiate_data_move(
                main, row, col, col == top_length - 1, row == left_length - 1
            )

            # Instantiate output movement structure
            instantiate_output_move(main, row, col, top_length)

    iter_limit = top_length + left_length + top_depth - 2
    iter_idx_size = bits_needed(iter_limit)
    instantiate_while_groups(main, iter_idx_size, iter_limit)

    for idx in range(top_length + left_length - 1):
        instantiate_idx_between(main, idx, idx + left_depth, iter_idx_size)

    # Generate the control and set the source map
    control, source_map = generate_control(
        main, top_length, top_depth, left_length, left_depth
    )
    main.control = control
    prog.program.meta = source_map


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
            "Need to pass either `FILE` or all of `"
            "-tl TOP_LENGTH -td TOP_DEPTH -ll LEFT_LENGTH -ld LEFT_DEPTH`"
        )

    prog = cb.Builder()
    pe(prog)
    create_systolic_array(
        prog,
        top_length=top_length,
        top_depth=top_depth,
        left_length=left_length,
        left_depth=left_depth,
    )

    prog.program.emit()
