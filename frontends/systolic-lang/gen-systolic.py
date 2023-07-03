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
    with comp.static_group("do_add", 1):
        add.left = acc.out
        add.right = mul.out
        acc.in_ = add.out
        acc.write_en = this.mul_ready

    with comp.static_group("do_mul", 1):
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
    with comp.static_group(init_name, 1):
        # Initialize the indexor to 0
        reg.in_ = 0
        reg.write_en = 1

    upd_name = NAME_SCHEME["index update"].format(prefix=prefix)
    with comp.static_group(upd_name, 1):
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
    with comp.static_group(group_name, 1):
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
        with comp.static_group(group_name, 1):
            dst_reg.in_ = src_reg.out
            dst_reg.write_en = 1

    if not down_edge:
        group_name = NAME_SCHEME["register move down"].format(pe=name)
        src_reg = comp.get_cell(f"top_{row}_{col}")
        dst_reg = comp.get_cell(f"top_{row + 1}_{col}")
        with comp.static_group(group_name, 1):
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
    with comp.static_group(group_name, 1):
        out.addr0 = idx
        out.write_data = pe.out
        out.write_en = 1


def gen_schedules(top_length, top_depth, left_length, left_depth):
    """
    Generates 4 arrays that are the same size as the output (systolic) array
    Each entry in the array has tuple [start, end) that indicates the cycles that
    they are active
    `update_sched` contains when to update the indices of the input memories and feed
    them into the systolic array
    `pe_fill_sched` contains when to invoke PE but not accumulate (bc the multipliers
    are not ready with an output yet)
    `pe_accum_sched` contains when to invoke PE and accumulate (bc the multipliers
    are ready with an output)
    `pe_move_sched` contains when to "move" the PE (i.e., pass data)
    """
    update_sched = np.zeros((left_length, top_length), dtype=object)
    pe_fill_sched = np.zeros((left_length, top_length), dtype=object)
    pe_accum_sched = np.zeros((left_length, top_length), dtype=object)
    pe_move_sched = np.zeros((left_length, top_length), dtype=object)
    for row in range(0, left_length):
        for col in range(0, top_length):
            pos = row + col
            update_sched[row][col] = (pos, pos + left_depth)
            pe_fill_sched[row][col] = (pos + 1, pos + min(4, left_depth) + 1)
            pe_accum_sched[row][col] = (pos + 5, pos + left_depth + 5)
            pe_move_sched[row][col] = (pos + 1, pos + left_depth + 1)
    return (update_sched, pe_fill_sched, pe_accum_sched, pe_move_sched)


def accum_nec_ranges(nec_ranges, schedule):
    """
    Essentially creates a set that contains all of the idx ranges that
    we need to check for (e.g., [1,3) [2,4)] in order to realize
    the schedule

    nec_ranges is a set of tuples.
    schedule is a 2d array with tuple (start,end) entries.
    Adds all intervals (start,end) in schedule to nec_ranges if the it's
    not already in nec_ranges
    """
    for r in schedule:
        for c in r:
            nec_ranges.add(c)
    return nec_ranges


def instantiate_idx_groups(comp: cb.ComponentBuilder, width, limit):
    """
    Builds groups that instantiate idx to 0 and increment idx
    """
    idx = comp.reg("idx", width)
    add = comp.add("idx_add", width)
    with comp.static_group("init_idx", 1):
        idx.in_ = 0
        idx.write_en = 1
    with comp.static_group("incr_idx", 1):
        add.left = idx.out
        add.right = 1
        idx.in_ = add.out
        idx.write_en = 1


def instantiate_idx_between(comp: cb.ComponentBuilder, lo, hi, width) -> list:
    """
    Instantiates a static group and register called "idx_between_{lo}_{hi}_reg/group"
    that should output whether idx is between [lo, hi). That is, whether lo <= idx < hi.

    Note: If you're trying to understand why this works, we are checking `idx_add` which
    is one higher than idx. This offsets the cycle it takes to update the register.
    """
    idx_add = comp.get_cell("idx_add")
    reg_str = f"idx_between_{lo}_{hi}_reg"
    comb_str = f"idx_between_{lo}_{hi}_comb"
    group_str = f"idx_between_{lo}_{hi}_group"
    index_lt = f"index_lt_{hi}"
    index_ge = f"index_ge_{lo}"
    reg = comp.reg(reg_str, 1)
    # if lo == 0, then only need to check if reg < hi
    if lo == 0:
        lt = comp.lt(comb_str, width)
        with comp.static_group(group_str, 1):
            lt.left = idx_add.out
            lt.right = hi
            reg.in_ = lt.out
            reg.write_en = 1
    # need to check if reg >= lo and reg < hi
    else:
        lt = comp.lt(index_lt, width)
        ge = comp.ge(index_ge, width)
        and_ = comp.and_(comb_str, 1)
        with comp.static_group(group_str, 1):
            ge.left = idx_add.out
            ge.right = lo
            lt.left = idx_add.out
            lt.right = hi
            and_.left = ge.out
            and_.right = lt.out
            reg.in_ = and_.out
            reg.write_en = 1


def instantiate_init_group(comp: cb.ComponentBuilder, lo, hi):
    """
    Builds a group to set initial state for register idx_between_{lo}_{hi}_reg.
    """
    # if lo == 0, then the idx will initially be in between the interval, so
    # need to set idx_between to high
    start_hi = 1 if lo == 0 else 0
    idx_between = comp.get_cell(f"idx_between_{lo}_{hi}_reg")
    with comp.static_group(f"init_idx_between_{lo}_{hi}", 1):
        idx_between.in_ = start_hi
        idx_between.write_en = 1


def get_memory_updates(row, col):
    """
    Gets the memory moves and memory idx updates for (row,col)
    This is how we coordinate feeding the memories into the systolic array
    """
    movers = []
    if col == 0:
        movers.append(NAME_SCHEME["memory move"].format(prefix=f"l{row}"))
        movers.append(NAME_SCHEME["index update"].format(prefix=f"l{row}"))
    if row == 0:
        movers.append(NAME_SCHEME["memory move"].format(prefix=f"t{col}"))
        movers.append(NAME_SCHEME["index update"].format(prefix=f"t{col}"))
    mover_enables = [py_ast.Enable(name) for name in movers]
    return mover_enables


def get_pe_moves(r, c, top_length, left_length):
    """
    Gets the PE moves for the PE at (r,c)
    """
    pe_moves = []
    if r < left_length - 1:
        pe_moves.append(NAME_SCHEME["register move down"].format(pe=f"pe_{r}_{c}"))
    if c < top_length - 1:
        pe_moves.append(NAME_SCHEME["register move right"].format(pe=f"pe_{r}_{c}"))
    pe_enables = [py_ast.Enable(name) for name in pe_moves]
    return pe_enables


def get_pe_invoke(r, c, top_length, left_length, mul_ready):
    """
    gets the PE invokes for the PE at (r,c). mul_ready signals whether 1 or 0
    should be passed into mul_ready
    """
    return py_ast.StaticInvoke(
        id=py_ast.CompVar(f"pe_{r}_{c}"),
        in_connects=[
            ("top", py_ast.CompPort(py_ast.CompVar(f"top_{r}_{c}"), "out")),
            (
                "left",
                py_ast.CompPort(py_ast.CompVar(f"left_{r}_{c}"), "out"),
            ),
            (
                "mul_ready",
                py_ast.ConstantPort(1, mul_ready),
            ),
        ],
        out_connects=[],
    )


def execute_if_between(comp: cb.ComponentBuilder, start, end, body):
    """
    body is a list of control stmts
    if body is empty, return an empty list
    otherwise, builds an if stmt that executes body in parallel if
    idx is between start and end
    """
    if not body:
        return []
    if_cell = comp.get_cell(f"idx_between_{start}_{end}_reg")
    return [
        cb.static_if(
            if_cell.out,
            py_ast.StaticParComp(body),
        )
    ]


def generate_control(
    comp: cb.ComponentBuilder,
    top_length,
    top_depth,
    left_length,
    left_depth,
    update_sched,
    fill_sched,
    accum_sched,
    move_sched,
    nec_ranges,
):
    """
    Logically, control performs the following actions:
    1. Initialize all the memory indexors and idx and idx_between
    registers at the start
    2. Build a static repeat with a one cycle body that:
        a. Updates memory indices if needed/feeds memory into systolic array.
        b. Invokes the PEs correctly (mul_ready should only be hi if
        the multiplier's values are ready).
        c. Move the data needed by each PE
    3. Writes the PE values into external memory
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
    for r in range(left_length):
        for c in range(top_length):
            # build 4 if stmts for the 4 schedules that we need to account for
            input_mem_updates = execute_if_between(
                comp,
                update_sched[r][c][0],
                update_sched[r][c][1],
                get_memory_updates(r, c),
            )
            pe_fills = execute_if_between(
                comp,
                fill_sched[r][c][0],
                fill_sched[r][c][1],
                [get_pe_invoke(r, c, top_length, left_length, 0)],
            )
            pe_moves = execute_if_between(
                comp,
                move_sched[r][c][0],
                move_sched[r][c][1],
                get_pe_moves(r, c, top_length, left_length),
            )
            pe_accums = execute_if_between(
                comp,
                accum_sched[r][c][0],
                accum_sched[r][c][1],
                [get_pe_invoke(r, c, top_length, left_length, 1)],
            )
            tag = counter()
            source_map[
                tag
            ] = f"pe_{r}_{c} filling: [{fill_sched[r][c][0]},{fill_sched[r][c][1]}) \
accumulating: [{accum_sched[r][c][0]} {accum_sched[r][c][1]})"
            pe_control = input_mem_updates + pe_fills + pe_moves + pe_accums
            control_stmts.append(py_ast.StaticParComp(pe_control))
    for start, end in nec_ranges:
        # build the control stmts that assign correct values to
        # idx_between_{start}_{end}_reg, which is what the if stmts above^ rely on
        incr_stmts.append(py_ast.Enable(f"idx_between_{start}_{end}_group"))

    repeat_body = py_ast.StaticParComp(
        [py_ast.StaticParComp(control_stmts), py_ast.StaticParComp(incr_stmts)]
    )

    # build the static repeat
    # num repeats = (top_length - 1) + (left_length - 1) + (top_depth - 1) + 5 + 1
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

    (update_sched, fill_sched, accum_sched, move_sched) = gen_schedules(
        top_length, top_depth, left_length, left_depth
    )
    nec_ranges = set()
    accum_nec_ranges(nec_ranges, update_sched)
    accum_nec_ranges(nec_ranges, fill_sched)
    accum_nec_ranges(nec_ranges, accum_sched)
    accum_nec_ranges(nec_ranges, move_sched)

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

    iter_limit = top_length + left_length + top_depth + 3
    iter_idx_size = bits_needed(iter_limit)
    # instantiate groups that initialize idx to 0 and increment it
    instantiate_idx_groups(main, iter_idx_size, iter_limit)

    for start, end in nec_ranges:
        # create the groups that create for idx_in_between registers
        instantiate_idx_between(main, start, end, iter_idx_size)
        instantiate_init_group(main, start, end)

    # Generate the control and set the source map
    control, source_map = generate_control(
        main,
        top_length,
        top_depth,
        left_length,
        left_depth,
        update_sched,
        fill_sched,
        accum_sched,
        move_sched,
        nec_ranges,
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
