#!/usr/bin/env python3

import numpy as np
from gen_pe import pe, PE_NAME, BITWIDTH
import calyx.builder as cb
from calyx import py_ast
from calyx.utils import bits_needed

# Global constant for the current bitwidth.
DEPTH = "depth"
SYSTOLIC_ARRAY_COMP = "systolic_array_comp"

# Naming scheme for generated groups. Used to keep group names consistent
# across structure and control.
NAME_SCHEME = {
    # Indexing into the memory
    "index name": "{prefix}_idx",
    "index init": "{prefix}_idx_init",
    "index update": "{prefix}_idx_update",
    # Move data from main memories
    "memory move": "{prefix}_move",
    "out write": "{pe}_out_write",
    # Move data between internal registers
    "register move down": "{pe}_down_move",
    "register move right": "{pe}_right_move",
    # Output signals
    "systolic valid signal": "r{row_num}_valid",
    "systolic value signal": "r{row_num}_value",
    "systolic idx signal": "r{row_num}_idx",
    # "Index between" registers to help with scheduling
    "idx between reg": "idx_between_{lo}_{hi}_reg",
    "idx between group": "idx_between_{lo}_{hi}_group",
    "idx between init": "init_idx_between_{lo}_{hi}",
}


class CalyxAdd:
    """
    A class that represents addition in Calyx between a port and a constant
    """

    def __init__(self, port, const):
        self.port = port
        self.const = const

    def __eq__(self, other):
        if type(other) != CalyxAdd:
            return False
        return (
            cb.ExprBuilder.unwrap(self.port) == cb.ExprBuilder.unwrap(other.port)
            and self.const == other.const
        )

    def __hash__(self):
        return hash(self.const)

    def __str__(self):
        return (
            str(cb.ExprBuilder.unwrap(self.port).item.id.name)
            + "_plus_"
            + str(self.const)
        )

    def build_group(self, comp: cb.ComponentBuilder) -> str:
        """
        Builds a static Calyx group (latency 1) that implements `self`
        Note that we avoid creating duplicate groups.
        Returns the group name
        """
        group_name = str(self) + "_group"
        if comp.try_get_group(group_name) is None:
            add = comp.add(BITWIDTH, str(self))
            with comp.static_group(group_name, 1):
                add.left = self.port
                add.right = self.const
        return group_name


def add_systolic_output_params(comp: cb.ComponentBuilder, row_num, addr_width):
    """
    Add output arguments to systolic array component `comp` for row `row_num`.
    The ouptut arguments alllow the systolic array to expose its outputs for `row_num`
    without writing to memory (e.g., r0_valid, r0_value, r0_idx).
    """
    cb.add_comp_params(
        comp,
        input_ports=[],
        output_ports=[
            (NAME_SCHEME["systolic valid signal"].format(row_num=row_num), 1),
            (NAME_SCHEME["systolic value signal"].format(row_num=row_num), BITWIDTH),
            (NAME_SCHEME["systolic idx signal"].format(row_num=row_num), addr_width),
        ],
    )


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
    cb.add_read_mem_params(comp, name, data_width=BITWIDTH, addr_width=idx_width)
    this = comp.this()
    addr0_port = this.port(name + "_addr0")
    read_data_port = this.port(name + "_read_data")
    # Instantiate the indexing register
    idx = instantiate_indexor(comp, name, idx_width)
    # Register to save the value from the memory. Defined by [[instantiate_pe]].
    target = comp.get_cell(target_reg)
    group_name = NAME_SCHEME["memory move"].format(prefix=name)
    with comp.static_group(group_name, 1) as g:
        g.asgn(addr0_port, idx.out)
        target.in_ = read_data_port
        target.write_en = 1


def instantiate_pe(comp: cb.ComponentBuilder, row: int, col: int):
    """
    Instantiate the PE and all the registers connected to it.
    """
    # Add all the required cells.
    comp.cell(f"pe_{row}_{col}", py_ast.CompInst(PE_NAME, []))
    comp.reg(f"top_{row}_{col}", BITWIDTH)
    comp.reg(f"left_{row}_{col}", BITWIDTH)


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
    add = comp.add(width, f"{prefix}_add")

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


def instantiate_output_move(comp: cb.ComponentBuilder, row, col):
    """
    Generates groups to move the final value from a PE to the output ports,
    e.g., writes the value of the PE to `this.r{row}_value_port`
    """
    group_name = NAME_SCHEME["out write"].format(pe=f"pe_{row}_{col}")
    pe = comp.get_cell(f"pe_{row}_{col}")
    this = comp.this()
    valid_port = this.port(NAME_SCHEME["systolic valid signal"].format(row_num=row))
    value_port = this.port(NAME_SCHEME["systolic value signal"].format(row_num=row))
    idx_port = this.port(NAME_SCHEME["systolic idx signal"].format(row_num=row))
    with comp.static_group(group_name, 1) as g:
        g.asgn(valid_port, 1)
        g.asgn(value_port, pe.out)
        g.asgn(idx_port, col)


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


def get_pe_invoke(r, c, mul_ready):
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


def init_runtime_vals(comp: cb.ComponentBuilder, depth_port, partial_iter_limit):
    """
    Builds group that instantiates the dynamic/runtime values for the systolic
    array: its depth and iteration limit/count (since its iteration limit depends on
    its depth).
    iteration limit = depth + partial_iter_limit
    """
    min_depth_4 = comp.reg("min_depth_4", BITWIDTH)
    lt_depth_4 = comp.lt(BITWIDTH, "lt_depth_4")
    iter_limit = comp.reg("iter_limit", BITWIDTH)
    iter_limit_add = comp.add(BITWIDTH, "iter_limit_add")
    with comp.static_group("init_min_depth", 1):
        lt_depth_4.left = depth_port
        lt_depth_4.right = 4
        min_depth_4.in_ = lt_depth_4.out @ depth_port
        min_depth_4.in_ = ~lt_depth_4.out @ 4
        min_depth_4.write_en = 1
    with comp.static_group("init_iter_limit", 1):
        iter_limit_add.left = partial_iter_limit
        iter_limit_add.right = depth_port
        iter_limit.in_ = iter_limit_add.out
        iter_limit.write_en = 1


def instantiate_while_groups(comp: cb.ComponentBuilder):
    """
    Builds groups that instantiate idx to 0 and increment idx.
    Also builds groups that set cond_reg to 1 (runs before the while loop)
    and that sets cond_reg to (idx + 1 < iter_limit).
    """
    idx = comp.reg("idx", BITWIDTH)
    add = comp.add(BITWIDTH, "idx_add")
    cond_reg = comp.reg("cond_reg", 1)
    iter_limit = comp.get_cell("iter_limit")
    lt_iter_limit = comp.lt(BITWIDTH, "lt_iter_limit")

    with comp.static_group("init_idx", 1):
        idx.in_ = 0
        idx.write_en = 1
    with comp.static_group("incr_idx", 1):
        add.left = idx.out
        add.right = 1
        idx.in_ = add.out
        idx.write_en = 1
    with comp.static_group("init_cond_reg", 1):
        cond_reg.in_ = 1
        cond_reg.write_en = 1
    with comp.static_group("write_cond_reg", 1):
        lt_iter_limit.left = add.out
        lt_iter_limit.right = iter_limit.out
        cond_reg.in_ = lt_iter_limit.out
        cond_reg.write_en = 1


def instantiate_calyx_adds(comp, nec_ranges) -> list:
    """
    Instantiates the CalyxAdd objects to adders and actual groups that perform the
    specified add.
    Returns a list of all the group names that we created.
    """
    calyx_add_groups = set()
    for lo, hi in nec_ranges:
        if type(lo) == CalyxAdd:
            group_name = lo.build_group(comp)
            calyx_add_groups.add(group_name)
        if type(hi) == CalyxAdd:
            group_name = hi.build_group(comp)
            calyx_add_groups.add(group_name)
    group_list = list(calyx_add_groups)
    # sort for testing purposes
    group_list.sort()
    return group_list


def instantiate_idx_between(comp: cb.ComponentBuilder, lo, hi) -> list:
    """
    Instantiates a static group and register called "idx_between_{lo}_{hi}_reg/group"
    that should output whether idx is between [lo, hi). That is, whether lo <= idx < hi.

    Note: If you're trying to understand why this works, we are checking `idx_add` which
    is one higher than idx. This offsets the cycle it takes to update the register.
    """
    if type(hi) == CalyxAdd:
        hi_value = comp.get_cell(str(hi)).port("out")
    else:
        hi_value = hi
    if type(lo) == CalyxAdd:
        lo_value = comp.get_cell(str(lo)).port("out")
    else:
        lo_value = lo
    reg_str = NAME_SCHEME["idx between reg"].format(lo=lo, hi=hi)
    group_str = NAME_SCHEME["idx between group"].format(lo=lo, hi=hi)
    index_lt = f"index_lt_{hi}"
    index_ge = f"index_ge_{lo}"
    reg = comp.reg(reg_str, 1)
    idx_add = comp.get_cell("idx_add")
    # If no upper bound, then only need to check reg >= lo
    lt = (
        comp.get_cell(index_lt)
        if comp.try_get_cell(index_lt) is not None
        else comp.lt(BITWIDTH, index_lt)
    )
    # if lo == 0, then only need to check if reg < hi
    if type(lo) == int and lo == 0:
        with comp.static_group(group_str, 1):
            lt.left = idx_add.out
            lt.right = hi_value
            reg.in_ = lt.out
            reg.write_en = 1
    # need to check if reg >= lo and reg < hi
    else:
        ge = (
            comp.get_cell(index_ge)
            if comp.try_get_cell(index_ge) is not None
            else comp.ge(BITWIDTH, index_ge)
        )
        and_ = comp.and_(1, f"idx_between_{lo}_{hi}_comb")
        with comp.static_group(group_str, 1):
            ge.left = idx_add.out
            ge.right = lo_value
            lt.left = idx_add.out
            lt.right = hi_value
            and_.left = ge.out
            and_.right = lt.out
            reg.in_ = and_.out
            reg.write_en = 1


def init_idx_between(comp: cb.ComponentBuilder, lo, hi):
    """
    Builds a group to set initial state for register idx_between_{lo}_{hi}_reg.
    """
    # if lo == 0, then the idx will initially be in between the interval, so
    # need to set idx_between to high
    start_hi = 1 if lo == 0 else 0
    idx_between = comp.get_cell(NAME_SCHEME["idx between reg"].format(lo=lo, hi=hi))
    with comp.static_group(NAME_SCHEME["idx between init"].format(lo=lo, hi=hi), 1):
        idx_between.in_ = start_hi
        idx_between.write_en = 1


def accum_nec_ranges(nec_ranges, schedule):
    """
    Essentially creates a set that contains all of the idx ranges that
    we need to check for (e.g., [1,3) [2,4)] in order to realize
    the schedule

    nec_ranges is a set of tuples.
    schedule is either a 2d array or 1d array with tuple (start,end) entries.
    Adds all intervals (start,end) in schedule to nec_ranges if the it's
    not already in nec_ranges.
    """
    if schedule.ndim == 1:
        for r in schedule:
            nec_ranges.add(r)
    elif schedule.ndim == 2:
        for r in schedule:
            for c in r:
                nec_ranges.add(c)
    else:
        raise Exception("accum_nec_ranges expects only 1d or 2d arrays")
    return nec_ranges


def gen_schedules(
    top_length,
    top_depth,
    left_length,
    left_depth,
    comp: cb.ComponentBuilder,
):
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
    `pe_write_sched` contains when to "write" the PE value into the output ports
    (e.g., this.r0_valid)
    """
    depth_port = comp.this().depth
    min_depth_4_port = comp.get_cell("min_depth_4").port("out")
    schedules = {}
    update_sched = np.zeros((left_length, top_length), dtype=object)
    pe_fill_sched = np.zeros((left_length, top_length), dtype=object)
    pe_accum_sched = np.zeros((left_length, top_length), dtype=object)
    pe_move_sched = np.zeros((left_length, top_length), dtype=object)
    # will only actually use one of the following two schedules
    pe_write_sched = np.zeros((left_length, top_length), dtype=object)
    for row in range(0, left_length):
        for col in range(0, top_length):
            pos = row + col
            update_sched[row][col] = (pos, CalyxAdd(depth_port, pos))
            pe_fill_sched[row][col] = (pos + 1, CalyxAdd(min_depth_4_port, pos + 1))
            pe_accum_sched[row][col] = (pos + 5, CalyxAdd(depth_port, pos + 5))
            pe_move_sched[row][col] = (pos + 1, CalyxAdd(depth_port, pos + 1))
            pe_write_sched[row][col] = (
                CalyxAdd(depth_port, pos + 5),
                CalyxAdd(depth_port, pos + 6),
            )
    schedules["update_sched"] = update_sched
    schedules["fill_sched"] = pe_fill_sched
    schedules["accum_sched"] = pe_accum_sched
    schedules["move_sched"] = pe_move_sched
    schedules["write_sched"] = pe_write_sched
    return schedules


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
    schedules,
    calyx_add_groups,
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
        + [
            py_ast.Enable("init_idx"),
            py_ast.Enable("init_min_depth"),
            py_ast.Enable("init_cond_reg"),
            py_ast.Enable("init_iter_limit"),
        ]
        + [py_ast.Enable(f"init_idx_between_{lo}_{hi}") for (lo, hi) in nec_ranges]
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
    incr_stmts = [py_ast.Enable("incr_idx"), py_ast.Enable("write_cond_reg")]
    for r in range(left_length):
        for c in range(top_length):
            # build 4 if stmts for the 4 schedules that we need to account for
            input_mem_updates = execute_if_between(
                comp,
                schedules["update_sched"][r][c][0],
                schedules["update_sched"][r][c][1],
                get_memory_updates(r, c),
            )
            pe_fills = execute_if_between(
                comp,
                schedules["fill_sched"][r][c][0],
                schedules["fill_sched"][r][c][1],
                [get_pe_invoke(r, c, 0)],
            )
            pe_moves = execute_if_between(
                comp,
                schedules["move_sched"][r][c][0],
                schedules["move_sched"][r][c][1],
                get_pe_moves(r, c, top_length, left_length),
            )
            pe_accums = execute_if_between(
                comp,
                schedules["accum_sched"][r][c][0],
                schedules["accum_sched"][r][c][1],
                [get_pe_invoke(r, c, 1)],
            )
            output_writes = execute_if_between(
                comp,
                schedules["write_sched"][r][c][0],
                schedules["write_sched"][r][c][1],
                [py_ast.Enable(NAME_SCHEME["out write"].format(pe=f"pe_{r}_{c}"))],
            )
            pe_control = (
                input_mem_updates + pe_fills + pe_moves + pe_accums + output_writes
            )
            control_stmts.append(py_ast.StaticParComp(pe_control))
            # providing metadata
            tag = counter()
            source_map[
                tag
            ] = f"pe_{r}_{c} filling: [{schedules['fill_sched'][r][c][0]},\
{schedules['fill_sched'][r][c][1]}), \
accumulating: [{schedules['accum_sched'][r][c][0]} \
{schedules['accum_sched'][r][c][1]}), \
writing: [{schedules['write_sched'][r][c][0]} \
{schedules['write_sched'][r][c][1]})"

    # handles the coordination so that `idx_if_between` statements work correctly `
    for start, end in nec_ranges:
        # build the control stmts that assign correct values to
        # idx_between_{start}_{end}_reg, which is what the if stmts above^ rely on
        incr_stmts.append(py_ast.Enable(f"idx_between_{start}_{end}_group"))
    for calyx_add_group in calyx_add_groups:
        incr_stmts.append(py_ast.Enable(calyx_add_group))
    while_ctrl = [py_ast.StaticParComp(control_stmts), py_ast.StaticParComp(incr_stmts)]
    while_body = py_ast.StaticParComp(while_ctrl)

    # build the while loop with condition cond_reg.
    # num repeats = (top_length - 1) + (left_length - 1) + (top_depth - 1) + 5 + 1
    cond_reg_port = comp.get_cell("cond_reg").port("out")
    while_loop = cb.while_(cond_reg_port, while_body)

    control.append(while_loop)

    return py_ast.SeqComp(stmts=control), source_map


def create_systolic_array(
    prog: cb.Builder,
    top_length,
    top_depth,
    left_length,
    left_depth,
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
    pe(prog)
    computational_unit = prog.component(SYSTOLIC_ARRAY_COMP)
    depth_port = computational_unit.input("depth", BITWIDTH)
    init_runtime_vals(computational_unit, depth_port, top_length + left_length + 4)

    schedules = gen_schedules(
        top_length, top_depth, left_length, left_depth, computational_unit
    )
    nec_ranges = set()
    for sched in schedules.values():
        accum_nec_ranges(nec_ranges, sched)
    calyx_add_groups = instantiate_calyx_adds(computational_unit, nec_ranges)

    for row in range(left_length):
        for col in range(top_length):
            # Instantiate the PEs and surronding registers
            instantiate_pe(computational_unit, row, col)

    # Instantiate all the memories
    for r in range(top_length):
        instantiate_memory(computational_unit, "top", r, top_depth)

    for col in range(left_length):
        instantiate_memory(computational_unit, "left", col, left_depth)

    # Instantiate output memory
    for i in range(left_length):
        add_systolic_output_params(computational_unit, i, bits_needed(top_length))

    # Instantiate all the PEs
    for row in range(left_length):
        for col in range(top_length):
            # Instantiate the mover fabric
            instantiate_data_move(
                computational_unit,
                row,
                col,
                col == top_length - 1,
                row == left_length - 1,
            )

            # Instantiate output movement structure, i.e., writes to
            # `computational_unit`'s output ports
            instantiate_output_move(computational_unit, row, col)

    # instantiate groups that handle cond_reg and idx variables
    instantiate_while_groups(computational_unit)
    for start, end in nec_ranges:
        # create the groups that create for idx_in_between registers
        instantiate_idx_between(computational_unit, start, end)
        init_idx_between(computational_unit, start, end)

    # Generate the control and set the source map
    control, source_map = generate_control(
        computational_unit,
        top_length,
        top_depth,
        left_length,
        left_depth,
        schedules,
        calyx_add_groups,
        nec_ranges,
    )
    computational_unit.control = control
    prog.program.meta = source_map
