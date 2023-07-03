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


# class PESchedule:
#     updates: tuple
#     pe_fills: tuple
#     pe_accums: tuple

#     def zeroed_instance():
#         return PESchedule((0, 0), (0, 0), (0, 0))

#     def __init__(self, updates: tuple, pe_fills: tuple, pe_accums: tuple):
#         self.updates = updates
#         self.pe_fills = pe_fills
#         self.pe_accums = pe_accums

#     def __repr__(self):
#         return f"PE SCHEDULE:\nupdates: {self.updates}\npe_fills: {self.pe_fills}\npe_accums: {self.pe_accums}\n"


def pe(prog: cb.Builder):
    comp = prog.component(name=PE_NAME, latency=1)
    comp.input("top", BITWIDTH)
    comp.input("left", BITWIDTH)
    comp.input("mul_ready", 1)
    comp.output("out", BITWIDTH)
    acc = comp.reg("acc", BITWIDTH)
    add = comp.add("add", BITWIDTH)
    # XXX: pipelined mult assumes 32 bit multiplication
    mul = comp.pipelined_mult("mul")

    this = comp.this()
    with comp.static_group("do_add", 1) as do_add:
        add.left = acc.out
        add.right = mul.out
        acc.in_ = add.out
        acc.write_en = this.mul_ready

    with comp.static_group("do_mul", 1) as do_mul:
        mul.left = this.top
        mul.right = this.left

    par = py_ast.StaticParComp([py_ast.Enable("do_add"), py_ast.Enable("do_mul")])

    with comp.continuous:
        this.out = acc.out

    comp.control += par


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
    with comp.static_group(init_name, 1) as init:
        # Initialize the indexor to 0
        reg.in_ = 0
        reg.write_en = 1

    upd_name = NAME_SCHEME["index update"].format(prefix=prefix)
    with comp.static_group(upd_name, 1) as upd:
        # Increment the indexor.
        add.left = 1
        add.right = reg.out
        reg.in_ = add.out
        reg.write_en = 1

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
    with comp.static_group(group_name, 1) as move:
        mem.addr0 = idx.out
        target.in_ = mem.read_data
        target.write_en = 1


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
        with comp.static_group(group_name, 1) as move:
            dst_reg.in_ = src_reg.out
            dst_reg.write_en = 1

    if not down_edge:
        group_name = NAME_SCHEME["register move down"].format(pe=name)
        src_reg = comp.get_cell(f"top_{row}_{col}")
        dst_reg = comp.get_cell(f"top_{row + 1}_{col}")
        with comp.static_group(group_name, 1) as move:
            dst_reg.in_ = src_reg.out
            dst_reg.write_en = 1


def instantiate_output_move(comp: cb.ComponentBuilder, row, col, cols):
    """
    Generates groups to move the final value from a PE into the output array.
    """
    group_name = NAME_SCHEME["out mem move"].format(pe=f"pe_{row}_{col}")
    idx = row * cols + col
    pe = comp.get_cell(f"pe_{row}_{col}")
    out = comp.get_cell(OUT_MEM)
    with comp.static_group(group_name, 1) as move:
        out.addr0 = idx
        out.write_data = pe.out
        out.write_en = 1


def gen_active_ranges(top_length, top_depth, left_length, left_depth):
    out = np.zeros((left_length, top_length), dtype=object)
    for row in range(0, left_length):
        for col in range(0, top_length):
            # PE at [row][col] is active for iterations [row_col, row + col + left_depth + 4)
            # (could've chosen top_depth instead since we know left_depth == top_depth)
            pos = row + col
            out[row][col] = (
                (pos, pos + left_depth),
                (pos + 1, pos + min(4, left_depth) + 1),
                (pos + 5, pos + left_depth + 5),
            )
    return out


def get_necessary_ranges(active_ranges):
    nec_ranges = set()
    for r in active_ranges:
        for pe_schedule in r:
            nec_ranges.add(pe_schedule[0])
            nec_ranges.add(pe_schedule[1])
            nec_ranges.add(pe_schedule[2])
    return nec_ranges


def instantiate_idx_groups(comp: cb.ComponentBuilder, width, limit):
    """
    comp: cb.ComponentBuilder, row, col, cols
    """
    idx = comp.reg("idx", width)
    add = comp.add(f"idx_add", width)
    with comp.static_group("init_idx", 1) as incr_grp:
        idx.in_ = 0
        idx.write_en = 1
    with comp.static_group("incr_idx", 1) as incr_grp:
        add.left = idx.out
        add.right = 1
        idx.in_ = add.out
        idx.write_en = 1


def instantiate_idx_between(comp: cb.ComponentBuilder, lo, hi, width) -> list:
    idx_add = comp.get_cell("idx_add")
    reg_str = f"idx_between_{lo}_{hi}_reg"
    comb_str = f"idx_between_{lo}_{hi}_comb"
    group_str = f"idx_between_{lo}_{hi}_group"
    index_lt = f"index_lt_{hi}"
    index_ge = f"index_ge_{lo}"
    if comp.try_get_group(group_str) is not None:
        return comp.get(group_str)
    reg = (
        comp.get_cell(reg_str)
        if comp.try_get_cell(reg_str) is not None
        else comp.reg(reg_str, 1)
    )
    if lo == 0:
        lt = (
            comp.get_cell(comb_str)
            if comp.try_get_cell(comb_str) is not None
            else comp.lt(comb_str, width)
        )
        with comp.static_group(group_str, 1) as idx_between:
            lt.left = idx_add.out
            lt.right = hi
            reg.in_ = lt.out
            reg.write_en = 1
    else:
        lt = (
            comp.get_cell(index_lt)
            if comp.try_get_cell(index_lt) is not None
            else comp.lt(index_lt, width)
        )
        ge = (
            comp.get_cell(index_ge)
            if comp.try_get_cell(index_ge) is not None
            else comp.ge(index_ge, width)
        )
        and_ = (
            comp.get_cell(comb_str)
            if comp.try_get_cell(comb_str) is not None
            else comp.and_(comb_str, 1)
        )
        with comp.static_group(group_str, 1) as idx_between:
            ge.left = idx_add.out
            ge.right = lo
            lt.left = idx_add.out
            lt.right = hi
            and_.left = ge.out
            and_.right = lt.out
            reg.in_ = and_.out
            reg.write_en = 1


def instantiate_init_group(comp: cb.ComponentBuilder, lo, hi, start_high):
    """
    comp: cb.ComponentBuilder, row, col, cols
    """
    idx = comp.get_cell(f"idx_between_{lo}_{hi}_reg")
    with comp.static_group(f"init_idx_between_{lo}_{hi}", 1) as incr_grp:
        idx.in_ = start_high
        idx.write_en = 1


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


def get_pe_invokes(idx, top_length, left_length, mul_ready):
    """
    get PE invokes
    """
    pe_invokes = []
    for x in range(left_length):
        for y in range(top_length):
            if idx == x + y:
                invoke = py_ast.StaticInvoke(
                    id=py_ast.CompVar(f"pe_{x}_{y}"),
                    in_connects=[
                        ("top", py_ast.CompPort(py_ast.CompVar(f"top_{x}_{y}"), "out")),
                        (
                            "left",
                            py_ast.CompPort(py_ast.CompVar(f"left_{x}_{y}"), "out"),
                        ),
                        (
                            "mul_ready",
                            py_ast.ConstantPort(1, mul_ready),
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
    comp: cb.ComponentBuilder,
    top_length,
    top_depth,
    left_length,
    left_depth,
    active_ranges,
    nec_ranges,
):
    """
    Logically, control performs the following actions:
    1. Initialize all the memory indexors at the start.
    2. For each time step in the schedule:
        a. Move the data required by PEs in this cycle.
        b. Update the memory indices if needed.
        c. Run the PEs that need to be active this cycle.
    """

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
        + [py_ast.Enable("init_idx")]
        + [py_ast.Enable(f"init_idx_between_{lo}_{hi}") for (lo, hi) in (nec_ranges)]
    )
    control.append(py_ast.StaticParComp(init_indices))

    # source_pos metadata init
    init_tag = 0
    source_map = {}

    def counter():
        nonlocal init_tag
        old = init_tag
        init_tag += 1
        return old

    # end source pos init

    control_stmts = []
    incr_stmts = [py_ast.Enable("incr_idx")]
    min_depth_4 = min(top_depth, 4)
    for r in range(left_length):
        for c in range(top_length):
            pe_schedule = active_ranges[r][c]
            updates_if_cell = comp.get_cell(
                f"idx_between_{ pe_schedule[0][0]}_{ pe_schedule[0][1]}_reg"
            )
            updates_body = get_movers(r + c, top_length, left_length) + get_idx_updates(
                r + c, top_length, left_length
            )
            updates_if = cb.static_if(
                updates_if_cell.out,
                py_ast.StaticParComp(updates_body),
            )
            pe_fills_body = get_pe_invokes(r + c, top_length, left_length, 0)
            pe_fills_if_cell = comp.get_cell(
                f"idx_between_{pe_schedule[1][0]}_{pe_schedule[1][1]}_reg"
            )
            pe_fills_if = cb.static_if(
                pe_fills_if_cell.out,
                py_ast.StaticParComp(pe_fills_body),
            )
            pe_accums_if_cell = comp.get_cell(
                f"idx_between_{pe_schedule[2][0]}_{pe_schedule[2][1]}_reg"
            )
            pe_accums_body = get_pe_invokes(r + c, top_length, left_length, 1)
            pe_accums_if = cb.static_if(
                pe_accums_if_cell.out,
                py_ast.StaticParComp(pe_accums_body),
            )
            control_stmts.append(
                py_ast.StaticParComp([updates_if, pe_fills_if, pe_accums_if])
            )
    for lo, hi in nec_ranges:
        incr_stmts.append(py_ast.Enable(f"idx_between_{lo}_{hi}_group"))

    repeat_body = py_ast.StaticParComp(
        [py_ast.StaticParComp(control_stmts), py_ast.StaticParComp(incr_stmts)]
    )

    static_repeat = cb.static_repeat(
        top_length + left_length + top_depth + 3, repeat_body
    )

    control.append(static_repeat)

    # Move all the results into output memory
    mover_groups = []
    for row in range(left_length):
        for col in range(top_length):
            mover_groups.append(
                py_ast.Enable(NAME_SCHEME["out mem move"].format(pe=f"pe_{row}_{col}"))
            )

    control.append(py_ast.StaticSeqComp(mover_groups))
    return py_ast.StaticSeqComp(stmts=control), source_map


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

    active_ranges = gen_active_ranges(top_length, top_depth, left_length, left_depth)
    nec_ranges = get_necessary_ranges(active_ranges)

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

    iter_limit = top_length + left_length + top_depth + 2
    iter_idx_size = bits_needed(iter_limit)
    instantiate_idx_groups(main, iter_idx_size, iter_limit)

    for lo, hi in nec_ranges:
        # idx_in_between should only start high if the interval includes 0
        start_hi = 1 if lo == 0 else 0
        instantiate_idx_between(main, lo, hi, iter_idx_size)
        instantiate_init_group(main, lo, hi, start_hi)

    # Generate the control and set the source map
    control, source_map = generate_control(
        main, top_length, top_depth, left_length, left_depth, active_ranges, nec_ranges
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
