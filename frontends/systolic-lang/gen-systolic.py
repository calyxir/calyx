#!/usr/bin/env python3

import numpy as np
import calyx.builder as cb
from calyx import py_ast
from calyx.utils import bits_needed
from fud.stages.verilator import numeric_types
from calyx.utils import float_to_fixed_point

# Global constant for the current bitwidth.
BITWIDTH = 32
INTWIDTH = 16
FRACWIDTH = 16
# Name of the ouput array
OUT_MEM = "out_mem"
PE_NAME = "mac_pe"
DEPTH = "depth"


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

    def __repr__(self):
        return (
            str(cb.ExprBuilder.unwrap(self.port).item.id.name)
            + "_plus_"
            + str(self.const)
        )

    def __str__(self):
        return (
            str(cb.ExprBuilder.unwrap(self.port).item.id.name)
            + "_plus_"
            + str(self.const)
        )


def pe(prog: cb.Builder, leaky_relu):
    comp = prog.component(name=PE_NAME, latency=1)
    comp.input("top", BITWIDTH)
    comp.input("left", BITWIDTH)
    comp.input("mul_ready", 1)
    comp.output("out", BITWIDTH)
    acc = comp.reg("acc", BITWIDTH)
    # Leaky relu means 32 bit signed fixed point operations.
    if leaky_relu:
        add = comp.fp_sop("adder", "add", BITWIDTH, INTWIDTH, FRACWIDTH)
        mul = comp.pipelined_fp_smult("mul", BITWIDTH, INTWIDTH, FRACWIDTH)
    # No leaky relu means integer operations
    else:
        add = comp.add(BITWIDTH, "add")
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


def add_read_mem_argument(comp: cb.ComponentBuilder, name, addr_width):
    """
    Add arguments to component `comp` if we want to read from a mem named `name` with
    width of `addr_width`
    """
    comp.input(f"{name}_read_data", BITWIDTH)
    comp.output(f"{name}_addr0", addr_width)


def add_write_mem_argument(comp: cb.ComponentBuilder, name, addr_width):
    """
    Add arguments to component `comp` if we want to write to a mem named `name` with
    width of `addr_width` inside `comp.`
    """
    comp.output(f"{name}_addr0", addr_width)
    comp.output(f"{name}_write_data", BITWIDTH)
    comp.output(f"{name}_write_en", 1)


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
    add_read_mem_argument(comp, name, idx_width)
    this = comp.this()
    addr0_port = cb.ExprBuilder.unwrap(this.port(name + "_addr0"))
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
    pe = comp.get_cell(f"pe_{row}_{col}")
    this = comp.this()
    mem_name = OUT_MEM + f"_{row}"
    addr0_port = cb.ExprBuilder.unwrap(this.port(mem_name + "_addr0"))
    write_data_port = cb.ExprBuilder.unwrap(this.port(mem_name + "_write_data"))
    write_en_port = cb.ExprBuilder.unwrap(this.port(mem_name + "_write_en"))
    with comp.static_group(group_name, 1) as g:
        g.asgn(addr0_port, col)
        g.asgn(write_data_port, pe.out)
        g.asgn(write_en_port, 1)


def instantiate_relu_cond_reg(
    comp: cb.ComponentBuilder,
    num_rows,
):
    """
    Writes into `cond_reg`, the condition register for the while loop.
    `cond_reg` basically checks whether the relu operation has finished yet
    for all rows of the array. If so, it sets `cond_reg` to lo. Otherwise it
    sets it to high.
    """
    cond_reg = comp.get_cell("cond_reg")
    cond_wire = comp.wire("cond_wire", 1)
    for r in range(num_rows):
        if r == 0:
            guard = comp.get_cell(f"relu_finished_reg_r{r}").port("out")
        else:
            guard = guard & comp.get_cell(f"relu_finished_reg_r{r}").port("out")
    with comp.static_group("write_cond_reg", 1):
        cond_wire.in_ = guard @ 1
        cond_reg.in_ = ~cond_wire.out @ 1
        cond_reg.in_ = cond_wire.out @ 0
        cond_reg.write_en = 1


def gen_schedules(
    top_length,
    top_depth,
    left_length,
    left_depth,
    leaky_relu,
    comp: cb.ComponentBuilder,
):
    """
    Generates 5 arrays that are the same size as the output (systolic) array
    Each entry in the array has tuple [start, end) that indicates the cycles that
    they are active
    `update_sched` contains when to update the indices of the input memories and feed
    them into the systolic array
    `pe_fill_sched` contains when to invoke PE but not accumulate (bc the multipliers
    are not ready with an output yet)
    `pe_accum_sched` contains when to invoke PE and accumulate (bc the multipliers
    are ready with an output)
    `pe_move_sched` contains when to "move" the PE (i.e., pass data)
    `pe_write_sched` contains when to "write" the PE value into memory (i.e., when
    the PE is "finished")
    `relu_sched` replaces `pe_write_sched` for Leaky Relu, and contains when to
    start the relu operation for a given row.
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
    relu_sched = np.zeros((left_length), dtype=object)
    for row in range(0, left_length):
        relu_sched[row] = (
            CalyxAdd(depth_port, row + 5),
            None,
        )
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
    # Only need one of relu_sched and write_sched
    if leaky_relu:
        schedules["relu_sched"] = relu_sched
    else:
        schedules["write_sched"] = pe_write_sched
    return schedules


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


def try_build_calyx_add(comp, obj):
    """
    Attempts to build an adder for obj, with name str(obj) and group name
    str(obj) + "_group" that adds obj.port and obj.const
    Returns true if we actually build it
    Returns false otherwise
    """
    if type(obj) == CalyxAdd:
        add_str = str(obj)
        if comp.try_get_cell(add_str) is None:
            add = comp.add(BITWIDTH, add_str)
            with comp.static_group(add_str + "_group", 1):
                add.left = obj.port
                add.right = obj.const
            return True
    return False


def instantiate_calyx_adds(comp, nec_ranges):
    """
    Instantiates the CalyxAdds objects to adders and actual groups that add things
    """
    depth_adders = []
    for lo, hi in nec_ranges:
        if try_build_calyx_add(comp, lo):
            depth_adders.append(str(lo) + "_group")
        if try_build_calyx_add(comp, hi):
            depth_adders.append(str(hi) + "_group")
    return depth_adders


def instantiate_idx_cond_groups(comp: cb.ComponentBuilder, leaky_relu):
    """
    Builds groups that instantiate idx to 0 and increment idx
    Also builds groups that set cond_reg to 1 (runs before the while loop)
    and that sets cond_reg to idx + 1 < iter_limit
    """
    idx = comp.reg("idx", BITWIDTH)
    add = comp.add(BITWIDTH, "idx_add")
    cond_reg = comp.reg("cond_reg", 1)
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
    # Only check iter_limit if not leaky_relu.
    # For leaky_relu we don't check iterations, we check if the relu
    # operations are finished yet
    if not leaky_relu:
        iter_limit = comp.get_cell("iter_limit")
        lt_iter_limit = comp.lt(BITWIDTH, "lt_iter_limit")
        with comp.static_group("lt_iter_limit_group", 1):
            lt_iter_limit.left = add.out
            lt_iter_limit.right = iter_limit.out
            cond_reg.in_ = lt_iter_limit.out
            cond_reg.write_en = 1


def init_dyn_vals(comp: cb.ComponentBuilder, depth_port, rem_iter_limit, leaky_relu):
    """
    Builds group that instantiates the dynamic/runtime values for the systolic
    array: its depth and iteration limit/count (since its iteration limit depends on
    its depth).
    If leaky_relu, we do not need to check iteration limit.
    """
    min_depth_4 = comp.reg("min_depth_4", BITWIDTH)
    lt_depth_4 = comp.lt(BITWIDTH, "lt_depth_4")
    with comp.static_group("init_min_depth", 1):
        lt_depth_4.left = depth_port
        lt_depth_4.right = 4
        min_depth_4.in_ = lt_depth_4.out @ depth_port
        min_depth_4.in_ = ~lt_depth_4.out @ 4
        min_depth_4.write_en = 1
    if not leaky_relu:
        iter_limit = comp.reg("iter_limit", BITWIDTH)
        iter_limit_add = comp.add(BITWIDTH, "iter_limit_add")
        with comp.static_group("init_iter_limit", 1):
            iter_limit_add.left = rem_iter_limit
            iter_limit_add.right = depth_port
            iter_limit.in_ = iter_limit_add.out
            iter_limit.write_en = 1


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
    idx_add = comp.get_cell("idx_add")
    reg_str = f"idx_between_{lo}_{hi}_reg"
    comb_str = f"idx_between_{lo}_{hi}_comb"
    group_str = f"idx_between_{lo}_{hi}_group"
    index_lt = f"index_lt_{str(hi)}"
    index_ge = f"index_ge_{str(lo)}"
    assert (
        not type(lo) is None
    ), "None Type Lower Bound not supported in instantiate_idx_between"
    # If no upper bound, then only need to check reg >= lo
    if hi is None:
        ge = (
            comp.get_cell(index_ge)
            if comp.try_get_cell(index_ge) is not None
            else comp.ge(BITWIDTH, index_ge)
        )
        with comp.static_group(group_str, 1):
            ge.left = idx_add.out
            ge.right = lo_value
    else:
        reg = comp.reg(reg_str, 1)
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
            and_ = comp.and_(1, comb_str)
            with comp.static_group(group_str, 1):
                ge.left = idx_add.out
                ge.right = lo_value
                lt.left = idx_add.out
                lt.right = hi_value
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
    # XXX(Caleb): assumed hi=None is used for a Relu computation, and therefore
    # idx_between_reg is not necessary.
    if hi is None:
        return
    idx_between = comp.get_cell(f"idx_between_{lo}_{hi}_reg")
    with comp.static_group(f"init_idx_between_{lo}_{hi}", 1):
        idx_between.in_ = start_hi
        idx_between.write_en = 1


def instantiate_relu_groups(comp: cb.ComponentBuilder, row, top_length):
    """
    Instantiates leaky relu groups that performs leaky relu on `row`.
    """

    # Helper function adds assignment wire.in = reg.out == col ? pe_{row}_{col}_out.
    def build_assignment(
        comp: cb.ComponentBuilder, group: cb.GroupBuilder, wire, register, row, col
    ):
        wire_in = wire.port("in")
        reg_out = register.port("out")
        pe_out = comp.get_cell(f"pe_{row}_{col}").port("out")
        group.asgn(
            wire_in,
            pe_out,
            reg_out == cb.ExprBuilder(py_ast.ConstantPort(BITWIDTH, col)),
        )

    # Current value we are performing relu on.
    cur_val = comp.wire(f"relu_r{row}_cur_val", BITWIDTH)
    # Current idx within the row for the value we are performing relu on.
    idx_reg = comp.reg(f"relu_r{row}_cur_idx", BITWIDTH)
    group_assigns = []
    group = comp.static_group(f"relu_r{row}_helper", 1)
    # assigning cur_val = value of PE at (row,idx_reg).
    for col in range(top_length):
        group_assigns.append(build_assignment(comp, group, cur_val, idx_reg, row, col))

    # Wire that tells us we are finished with relu operation for this row.
    relu_finished_wire = comp.wire(f"relu_finished_wire_r{row}", 1)
    # Register that holds the value of relu_finished_wire for later cycles.
    relu_finished_reg = comp.reg(f"relu_finished_reg_r{row}", 1)
    # Checks whether cur_val is > 0.
    cur_gt = comp.fp_sop(f"relu_r{row}_val_gt", "gt", BITWIDTH, INTWIDTH, FRACWIDTH)
    # Checks whether we should go onto the next entry in the row. This occurs
    # either when a) value is positive or b) multiply operation has finished.
    go_next = comp.wire(f"relu_r{row}_go_next", BITWIDTH)
    # Increments idx_reg.
    incr = comp.add(BITWIDTH, f"relu_r{row}_incr")
    # Performs multiplication for leaky relu.
    fp_mult = comp.fp_sop(
        f"relu_r{row}_val_mult", "mult_pipe", BITWIDTH, INTWIDTH, FRACWIDTH
    )
    this = comp.this()
    mem_name = OUT_MEM + f"_{row}"
    addr0_port = cb.ExprBuilder.unwrap(this.port(mem_name + "_addr0"))
    write_data_port = cb.ExprBuilder.unwrap(this.port(mem_name + "_write_data"))
    write_en_port = cb.ExprBuilder.unwrap(this.port(mem_name + "_write_en"))
    with comp.static_group(f"execute_relu_r{row}", 1) as g:
        # Check if the current value is positive or negative.
        cur_gt.left = cur_val.out
        cur_gt.right = 0

        # Handle incrementing the idx_reg.
        # Increment either when a) multiplication is done or b) cur value is positive
        incr.left = idx_reg.out
        incr.right = 1
        go_next.in_ = (fp_mult.done | cur_gt.out) @ 1
        idx_reg.in_ = go_next.out @ incr.out
        idx_reg.write_en = go_next.out @ 1

        # Perform the multiplication.
        # Get FP approximation of 0.01.
        fp_mult.left = numeric_types.FixedPoint(
            str(float_to_fixed_point(0.01, FRACWIDTH)), BITWIDTH, INTWIDTH, True
        ).unsigned_integer()
        fp_mult.right = cur_val.out
        fp_mult.go = ~go_next.out @ 1

        # Write to mem based on whether cur_valu >= 0
        g.asgn(write_en_port, 1, go_next.out)
        g.asgn(addr0_port, idx_reg.out)
        g.asgn(write_data_port, cur_val.out, cur_gt.out)
        g.asgn(write_data_port, fp_mult.out, ~cur_gt.out)

        # While loop logic. relu_finished when idx_reg == top_length - 1 & go_next,
        # i.e., when we're at the last index and about to "go to the next value".
        relu_finished_wire.in_ = (
            go_next.out
            & (
                idx_reg.out
                == cb.ExprBuilder(py_ast.ConstantPort(BITWIDTH, top_length - 1))
            )
        ) @ 1
        relu_finished_reg.in_ = relu_finished_wire.out @ 1
        relu_finished_reg.write_en_ = relu_finished_wire.out @ 1

    # Start relu when idx_ge row + depth + 5, i.e., when first value in row
    # is ready to be computed.
    relu_start_port = comp.get_cell(f"index_ge_depth_plus_{5 + row}").port("out")
    # relu_cond_wire coordinates when relu_cond_reg should be hi/lo
    relu_cond_wire = comp.wire(f"relu_cond_wire_r{row}", 1)
    # relu_cond_reg guards when we should perform the relu execution group defined
    # above.
    relu_cond_reg = comp.reg(f"relu_cond_reg_r{row}", 1)
    guard = relu_start_port & (~relu_finished_wire.out)
    with comp.static_group(f"check_relu_cond_r{row}", 1):
        relu_cond_wire.in_ = guard @ 1
        relu_cond_reg.in_ = relu_cond_wire.out @ 1
        relu_cond_reg.in_ = ~relu_cond_wire.out @ 0
        relu_cond_reg.write_en = ~relu_finished_reg.out @ 1


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


def execute_if_register(comp: cb.ComponentBuilder, register, body):
    """
    body is a list of control stmts
    if body is empty, return an empty list
    otherwise, builds an if stmt that executes body in parallel reg.out is high
    """
    if not body:
        return []
    return [
        cb.static_if(
            register.out,
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
    depth_adders,
    nec_ranges,
    leaky_relu,
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
        ]
        + [
            py_ast.Enable(f"init_idx_between_{lo}_{hi}")
            for (lo, hi) in filter(lambda x: x[1] is not None, nec_ranges)
        ]
    )
    if not leaky_relu:
        init_indices.append(py_ast.Enable("init_iter_limit"))

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
    if not leaky_relu:
        incr_stmts.append(py_ast.Enable("lt_iter_limit_group"))
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
                [get_pe_invoke(r, c, top_length, left_length, 0)],
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
                [get_pe_invoke(r, c, top_length, left_length, 1)],
            )
            if leaky_relu:
                pe_writes = []
            else:
                pe_writes = execute_if_between(
                    comp,
                    schedules["write_sched"][r][c][0],
                    schedules["write_sched"][r][c][1],
                    [
                        py_ast.Enable(
                            NAME_SCHEME["out mem move"].format(pe=f"pe_{r}_{c}")
                        )
                    ],
                )
            pe_control = input_mem_updates + pe_fills + pe_moves + pe_accums + pe_writes
            control_stmts.append(py_ast.StaticParComp(pe_control))
            # providing metadata
            tag = counter()
            source_map[
                tag
            ] = f"pe_{r}_{c} filling: [{schedules['fill_sched'][r][c][0]},\
{schedules['fill_sched'][r][c][1]}) accumulating: [{schedules['accum_sched'][r][c][0]} \
{schedules['accum_sched'][r][c][1]})"

    if leaky_relu:
        relu_execution = [py_ast.Enable("write_cond_reg")]
        for r in range(left_length):
            relu_execution += execute_if_register(
                comp,
                comp.get_cell(f"relu_cond_reg_r{r}"),
                [
                    py_ast.Enable(f"execute_relu_r{r}"),
                    py_ast.Enable(f"relu_r{r}_helper"),
                ],
            )
            relu_execution.append(py_ast.Enable(f"check_relu_cond_r{r}"))

    for start, end in nec_ranges:
        # build the control stmts that assign correct values to
        # idx_between_{start}_{end}_reg, which is what the if stmts above^ rely on
        incr_stmts.append(py_ast.Enable(f"idx_between_{start}_{end}_group"))
    for depth_adder_group in depth_adders:
        incr_stmts.append(py_ast.Enable(depth_adder_group))

    while_ctrl = [py_ast.StaticParComp(control_stmts), py_ast.StaticParComp(incr_stmts)]
    if leaky_relu:
        while_ctrl.append(py_ast.StaticParComp(relu_execution))
    while_body = py_ast.StaticParComp(while_ctrl)

    # build the while loop with condition cond_reg.
    # num repeats = (top_length - 1) + (left_length - 1) + (top_depth - 1) + 5 + 1
    cond_reg_port = comp.get_cell("cond_reg").port("out")
    while_loop = cb.while_(cond_reg_port, None, while_body)

    control.append(while_loop)

    return py_ast.SeqComp(stmts=control), source_map


def create_systolic_array(
    prog: cb.Builder,
    top_length,
    top_depth,
    left_length,
    left_depth,
    leaky_relu,
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

    computational_unit = prog.component("systolic_array_comp")
    depth_port = computational_unit.input("depth", BITWIDTH)
    init_dyn_vals(
        computational_unit, depth_port, top_length + left_length + 4, leaky_relu
    )

    schedules = gen_schedules(
        top_length, top_depth, left_length, left_depth, leaky_relu, computational_unit
    )
    nec_ranges = set()
    for sched in schedules.values():
        accum_nec_ranges(nec_ranges, sched)
    depth_adders = instantiate_calyx_adds(computational_unit, nec_ranges)

    for row in range(left_length):
        for col in range(top_length):
            # Instantiate the PEs and surronding registers
            instantiate_pe(computational_unit, row, col)

    # Instantiate all the memories
    for r in range(top_length):
        instantiate_memory(computational_unit, "top", r, top_depth)

    for col in range(left_length):
        instantiate_memory(computational_unit, "left", col, left_depth)

    idx_width = BITWIDTH
    # Instantiate output memory
    for i in range(left_length):
        add_write_mem_argument(computational_unit, OUT_MEM + f"_{i}", idx_width)

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

            # Instantiate output movement structure
            # Leaky relu will write into memories using different groups
            if not leaky_relu:
                instantiate_output_move(computational_unit, row, col, top_length)

    # instantiate groups that handle cond_reg and idx variables
    instantiate_idx_cond_groups(computational_unit, leaky_relu)
    for start, end in nec_ranges:
        # create the groups that create for idx_in_between registers
        instantiate_idx_between(computational_unit, start, end)
        instantiate_init_group(computational_unit, start, end)

    if leaky_relu:
        # Instantiate groups to compute Relu.
        for row in range(left_length):
            instantiate_relu_groups(computational_unit, row, top_length)
        # Write into the cond reg of the while loop.
        instantiate_relu_cond_reg(computational_unit, left_length)

    # Generate the control and set the source map
    control, source_map = generate_control(
        computational_unit,
        top_length,
        top_depth,
        left_length,
        left_depth,
        schedules,
        depth_adders,
        nec_ranges,
        leaky_relu,
    )
    computational_unit.control = control
    prog.program.meta = source_map

    # build the main component
    # instantaites the systolic array/computational_unit and the mems,
    # and then invokes it
    main = prog.component("main")
    systolic_array = main.cell("systolic_array", computational_unit)
    invoke_args = {}
    invoke_args["in_depth"] = py_ast.ConstantPort(BITWIDTH, left_depth)
    for r in range(top_length):
        name = f"t{r}"
        idx_width = bits_needed(top_depth)
        mem = main.mem_d1(
            name,
            BITWIDTH,
            top_depth,
            idx_width,
            is_external=True,
        )
        invoke_args[f"in_{name}_read_data"] = mem.read_data
        invoke_args[f"out_{name}_addr0"] = mem.addr0
    for col in range(left_length):
        name = f"l{col}"
        idx_width = bits_needed(left_depth)
        mem = main.mem_d1(
            name,
            BITWIDTH,
            left_depth,
            idx_width,
            is_external=True,
        )
        invoke_args[f"in_{name}_read_data"] = mem.read_data
        invoke_args[f"out_{name}_addr0"] = mem.addr0

    for i in range(left_length):
        name = OUT_MEM + f"_{i}"
        mem = main.mem_d1(
            name,
            BITWIDTH,
            top_length,
            BITWIDTH,
            is_external=True,
        )
        invoke_args[f"out_{name}_addr0"] = mem.addr0
        invoke_args[f"out_{name}_write_data"] = mem.write_data
        invoke_args[f"out_{name}_write_en"] = mem.write_en

    invoke = cb.invoke(systolic_array, **invoke_args)
    main.control = invoke


if __name__ == "__main__":
    import argparse
    import json

    parser = argparse.ArgumentParser(description="Process some integers.")
    parser.add_argument("file", nargs="?", type=str)
    parser.add_argument("-tl", "--top-length", type=int)
    parser.add_argument("-td", "--top-depth", type=int)
    parser.add_argument("-ll", "--left-length", type=int)
    parser.add_argument("-ld", "--left-depth", type=int)
    parser.add_argument("-r", "--leaky-relu", action="store_true")

    args = parser.parse_args()

    top_length, top_depth, left_length, left_depth, leaky_relu = (
        None,
        None,
        None,
        None,
        False,
    )

    fields = [args.top_length, args.top_depth, args.left_length, args.left_depth]
    if all(map(lambda x: x is not None, fields)):
        top_length = args.top_length
        top_depth = args.top_depth
        left_length = args.left_length
        left_depth = args.left_depth
        leaky_relu = args.leaky_relu
    elif args.file is not None:
        with open(args.file, "r") as f:
            spec = json.load(f)
            top_length = spec["top_length"]
            top_depth = spec["top_depth"]
            left_length = spec["left_length"]
            left_depth = spec["left_depth"]
            # default to not perform leaky_relu
            leaky_relu = spec.get("leaky_relu", False)
    else:
        parser.error(
            "Need to pass either `FILE` or all of `"
            "-tl TOP_LENGTH -td TOP_DEPTH -ll LEFT_LENGTH -ld LEFT_DEPTH`"
        )

    prog = cb.Builder()
    pe(prog, leaky_relu)
    create_systolic_array(
        prog,
        top_length=top_length,
        top_depth=top_depth,
        left_length=left_length,
        left_depth=left_depth,
        leaky_relu=leaky_relu,
    )

    prog.program.emit()
