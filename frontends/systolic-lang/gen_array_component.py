#!/usr/bin/env python3

import numpy as np
from gen_pe import pe, PE_NAME, BITWIDTH
import calyx.builder as cb
from calyx import py_ast
from calyx.utils import bits_needed
from systolic_arg_parser import SystolicConfiguration
from enum import Enum

# Global constant for the current bitwidth.
DEPTH = "depth"
SYSTOLIC_ARRAY_COMP = "systolic_array_comp"

# Naming scheme for generated groups. Used to keep group names consistent
# across structure and control.
NAME_SCHEME = {
    # Indexing into the memory
    "index name": "{prefix}_idx",
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

    def implement_add(self, comp: cb.ComponentBuilder) -> str:
        """
        Implements the `CalyxAdd` by creating an adder that adds the two values
        """
        if comp.try_get_cell(str(self)) is None:
            add = comp.add(BITWIDTH, str(self))
            with comp.continuous:
                add.left = self.port
                add.right = self.const


class ScheduleType(Enum):
    GE = 1
    LT = 2
    EQ = 3
    INTERVAL = 4


class ScheduleInstance:
    def __init__(self, type: ScheduleType, i1, i2=None):
        self.type = type
        self.i1 = i1
        self.i2 = i2
        if type == ScheduleType.INTERVAL and self.i2 == None:
            raise Exception(f"INTERVAL type must specify beginning and end")


class Schedule:
    def __init__(self):
        self.instances = set()
        self.mappings = {}

    def add_instances(self, name, schedule_instances):
        """ """
        self.mappings[name] = schedule_instances
        for schedule_instance in schedule_instances.flatten():
            self.instances.add(schedule_instance)

    def __instantiate_calyx_adds(self, comp) -> list:
        """ """
        for schedule_instance in self.instances:
            if type(schedule_instance.i1) == CalyxAdd:
                schedule_instance.i1.implement_add(comp)
            if type(schedule_instance.i2) == CalyxAdd:
                schedule_instance.i2.implement_add(comp)

    def __check_idx_eq(self, comp: cb.ComponentBuilder, idx_reg: cb.CellBuilder, eq):
        """
        Creates assignments to test if idx >= lo
        """
        if type(eq) == CalyxAdd:
            eq_value = comp.get_cell(str(eq)).port("out")
        else:
            eq_value = eq
        eq = comp.eq(BITWIDTH, f"index_eq_{eq}")
        with comp.continuous:
            eq.left = idx_reg.out
            eq.right = eq_value

    def __check_idx_lower_bound(
        self, comp: cb.ComponentBuilder, idx_reg: cb.CellBuilder, lo
    ):
        """
        Creates assignments to test if idx >= lo
        """
        if type(lo) == int and lo == 0:
            return
        if type(lo) == CalyxAdd:
            lo_value = comp.get_cell(str(lo)).port("out")
        else:
            lo_value = lo
        ge = comp.ge(BITWIDTH, f"index_ge_{lo}")
        with comp.continuous:
            ge.left = idx_reg.out
            ge.right = lo_value

    def __check_idx_upper_bound(
        self, comp: cb.ComponentBuilder, idx_reg: cb.CellBuilder, hi
    ):
        """
        Creates assignments to test if idx < hi
        """
        if type(hi) == CalyxAdd:
            hi_value = comp.get_cell(str(hi)).port("out")
        else:
            hi_value = hi
        lt = comp.lt(BITWIDTH, f"index_lt_{hi}")
        with comp.continuous:
            lt.left = idx_reg.out
            lt.right = hi_value

    def __check_idx_between(self, comp: cb.ComponentBuilder, lo, hi) -> list:
        """
        Creates assignments to check whether idx is between [lo, hi).
        That is, whether lo <= idx < hi.
        IMPORTANT: Assumes the lt and gt cells ahve already been created
        """
        # This is the name of the combinational cell that checks the condition
        idx_between_str = f"idx_between_{lo}_{hi}_comb"
        lt = comp.get_cell(f"index_lt_{hi}")
        # if lo == 0, then only need to check if reg < hi
        if type(lo) == int and lo == 0:
            # In this case, the `wire` cell is the cell checking the condition.
            wire = comp.wire(idx_between_str, 1)
            with comp.continuous:
                wire.in_ = lt.out
        # need to check if reg >= lo and reg < hi
        else:
            ge = comp.get_cell(f"index_ge_{lo}")
            # In this case, the `and` cell is the cell checking the condition.
            and_ = comp.and_(1, idx_between_str)
            with comp.continuous:
                and_.right = lt.out
                and_.left = ge.out

    def build_hardware(self, comp: cb.ComponentBuilder, idx_reg: cb.CellBuilder):
        """ """
        # instantiate groups that handles the idx variables
        ge_ranges = set()
        lt_ranges = set()
        eq_ranges = set()
        interval_ranges = set()
        for schedule_instance in self.instances:
            sched_type = schedule_instance.type
            if sched_type == ScheduleType.GE:
                ge_ranges.add(schedule_instance.i1)
            elif sched_type == ScheduleType.LT:
                lt_ranges.add(schedule_instance.i1)
            elif sched_type == ScheduleType.EQ:
                eq_ranges.add(schedule_instance.i1)
            elif sched_type == ScheduleType.INTERVAL:
                ge_ranges.add(schedule_instance.i1)
                lt_ranges.add(schedule_instance.i2)
                interval_ranges.add((schedule_instance.i1, schedule_instance.i2))
        self.__instantiate_calyx_adds(comp)
        for val in eq_ranges:
            self.__check_idx_eq(comp, idx_reg, val)
        for val in ge_ranges:
            self.__check_idx_lower_bound(comp, idx_reg, val)
        for val in lt_ranges:
            self.__check_idx_upper_bound(comp, idx_reg, val)
        for start, end in interval_ranges:
            self.__check_idx_between(comp, start, end)


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
    # Get the indexing value, taking into account offset
    # For example, for l2, we want to access idx-2 (since we want to wait two
    # cycles before we start feeding memories in)
    idx_val = get_indexor(comp, idx_width, offset=idx)
    # Register to save the value from the memory. Defined by [[instantiate_pe]].
    target = comp.get_cell(target_reg)
    group_name = NAME_SCHEME["memory move"].format(prefix=name)
    with comp.static_group(group_name, 1) as g:
        g.asgn(addr0_port, idx_val.out)
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


def get_indexor(comp: cb.ComponentBuilder, width: int, offset: int) -> cb.CellBuilder:
    """
    Gets (instantiates if needed) an indexor for accessing memory with offset
    `offset` (as compared to the iteration idx)
    """
    if comp.try_get_cell(f"idx_minus_{offset}_res") is None:
        idx = comp.get_cell("idx")
        # idx has width bitwidth
        sub = comp.sub(BITWIDTH, f"idx_minus_{offset}")
        sub_res = comp.slice(f"idx_minus_{offset}_res", BITWIDTH, width)
        with comp.continuous:
            sub.left = idx.out
            sub.right = offset
            sub_res.in_ = sub.out
        return sub_res
    else:
        return comp.get_cell(f"idx_minus_{offset}_res")


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
        src_reg = comp.get_cell(f"left_{row}_{col}")
        dst_reg = comp.get_cell(f"left_{row}_{col + 1}")
        with comp.continuous:
            dst_reg.in_ = src_reg.out
            dst_reg.write_en = 1

    if not down_edge:
        src_reg = comp.get_cell(f"top_{row}_{col}")
        dst_reg = comp.get_cell(f"top_{row + 1}_{col}")
        with comp.continuous:
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
    if row == 0:
        movers.append(NAME_SCHEME["memory move"].format(prefix=f"t{col}"))
    mover_enables = [py_ast.Enable(name) for name in movers]
    return mover_enables


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
                mul_ready,
            ),
        ],
        out_connects=[],
    )


def init_iter_limit(
    comp: cb.ComponentBuilder, depth_port, config: SystolicConfiguration
):
    """
    Builds group that instantiates the dynamic/runtime values for the systolic
    array: its depth and iteration limit/count (since its iteration limit depends on
    its depth).
    iteration limit = depth + partial_iter_limit
    """
    # Only need to initalize this group if
    if not config.static:
        partial_iter_limit = config.top_length + config.left_length + 4
        iter_limit = comp.reg("iter_limit", BITWIDTH)
        iter_limit_add = comp.add(BITWIDTH, "iter_limit_add")
        with comp.static_group("init_iter_limit", 1):
            iter_limit_add.left = partial_iter_limit
            iter_limit_add.right = depth_port
            iter_limit.in_ = iter_limit_add.out
            iter_limit.write_en = 1


def instantiate_idx_groups(comp: cb.ComponentBuilder, config: SystolicConfiguration):
    """
    Builds groups that instantiate idx to 0 and increment idx.
    Also builds groups that set cond_reg to 1 (runs before the while loop)
    and that sets cond_reg to (idx + 1 < iter_limit).
    """
    idx = comp.reg("idx", BITWIDTH)
    add = comp.add(BITWIDTH, "idx_add")

    with comp.static_group("init_idx", 1):
        idx.in_ = 0
        idx.write_en = 1
    with comp.static_group("incr_idx", 1):
        add.left = idx.out
        add.right = 1
        idx.in_ = add.out
        idx.write_en = 1
    if not config.static:
        iter_limit = comp.get_cell("iter_limit")
        lt_iter_limit = comp.lt(BITWIDTH, "lt_iter_limit")
        with comp.continuous:
            lt_iter_limit.left = idx.out
            lt_iter_limit.right = iter_limit.out


def gen_schedules(
    config: SystolicConfiguration,
    comp: cb.ComponentBuilder,
):
    """
    Generates 4 arrays that are the same size as the output (systolic) array
    Each entry in the array has tuple [start, end) that indicates the cycles that
    they are active
    `update_sched` contains when to update the indices of the input memories and feed
    them into the systolic array
    `pe_sched` contains when to invoke PE
    `pe_accum_cond` contains when to allow the PEs to accumulate (bc the multipliers
    are ready with an output)
    `pe_write_sched` contains when to "write" the PE value into the output ports
    (e.g., this.r0_valid)
    """

    def depth_plus_const(const: int):
        """
        Returns depth + const. If config.static, then this is an int.
        Otherwise, we need to perform a Calyx addition to figure this out.
        """
        if config.static:
            # return an int
            return config.get_contraction_dimension() + const
        else:
            # return a CalyxAdd object, whose value is determined after generation
            depth_port = comp.this().depth
            return CalyxAdd(depth_port, const)

    left_length, top_length = config.left_length, config.top_length
    update_sched = np.zeros((left_length, top_length), dtype=object)
    pe_sched = np.zeros((left_length, top_length), dtype=object)
    pe_accum_cond = np.zeros((left_length, top_length), dtype=object)
    pe_write_sched = np.zeros((left_length, top_length), dtype=object)
    for row in range(0, left_length):
        for col in range(0, top_length):
            pos = row + col
            update_sched[row][col] = ScheduleInstance(
                ScheduleType.INTERVAL, pos, depth_plus_const(pos)
            )
            pe_sched[row][col] = ScheduleInstance(
                ScheduleType.INTERVAL, pos + 1, depth_plus_const(pos + 5)
            )
            pe_accum_cond[row][col] = ScheduleInstance(ScheduleType.GE, pos + 5)
            pe_write_sched[row][col] = ScheduleInstance(
                ScheduleType.EQ, depth_plus_const(pos + 5)
            )
    schedule = Schedule()
    schedule.add_instances("update_sched", update_sched)
    schedule.add_instances("pe_sched", pe_sched)
    schedule.add_instances("pe_accum_cond", pe_accum_cond)
    schedule.add_instances("pe_write_sched", pe_write_sched)
    return schedule


def execute_if_between(comp: cb.ComponentBuilder, start, end, body):
    """
    body is a list of control stmts
    if body is empty, return an empty list
    otherwise, builds an if stmt that executes body in parallel if
    idx is between start and end
    """
    if not body:
        return []
    if_cell = comp.get_cell(f"idx_between_{start}_{end}_comb")
    return [
        cb.static_if(
            if_cell.out,
            py_ast.StaticParComp(body),
        )
    ]


def execute_if_eq(comp: cb.ComponentBuilder, val, body):
    """
    body is a list of control stmts
    if body is empty, return an empty list
    otherwise, builds an if stmt that executes body in parallel if
    idx is between start and end
    """
    if not body:
        return []
    if_cell = comp.get_cell(f"index_eq_{val}")
    return [
        cb.static_if(
            if_cell.out,
            py_ast.StaticParComp(body),
        )
    ]


def generate_control(
    comp: cb.ComponentBuilder, config: SystolicConfiguration, schedule
):
    """
    Logically, control performs the following actions:
    1. Initialize all the memory indexors and idx and idx_between
    registers at the start
    2. Build a static loop with a one cycle body that:
        a. Updates memory indices if needed/feeds memory into systolic array.
        b. Invokes the PEs correctly (mul_ready should only be hi if
        the multiplier's values are ready).
        c. Move the data needed by each PE
    3. Writes the PE values into output ports of the component when necessary
    """
    control = []
    top_length, left_length = config.top_length, config.left_length

    # Initialize the idx and iteration_limit.
    # We only need to initialize iteration_limit for dynamic configurations
    init_groups = [py_ast.Enable("init_idx")]
    if not config.static:
        init_groups += [py_ast.Enable("init_iter_limit")]
    control.append(py_ast.StaticParComp(init_groups))

    # source_pos metadata init
    init_tag = 0
    source_map = {}

    def counter():
        nonlocal init_tag
        old = init_tag
        init_tag += 1
        return old

    # end source pos init

    while_body_stmts = [py_ast.Enable("incr_idx")]
    for r in range(left_length):
        for c in range(top_length):
            # build 4 if stmts for the 4 schedules that we need to account for
            input_mem_updates = execute_if_between(
                comp,
                schedule.mappings["update_sched"][r][c].i1,
                schedule.mappings["update_sched"][r][c].i2,
                get_memory_updates(r, c),
            )
            pe_accum_thresh = schedule.mappings["pe_accum_cond"][r][c].i1
            pe_accum_cond = py_ast.CompPort(
                py_ast.CompVar(f"index_ge_{pe_accum_thresh}"), "out"
            )
            pe_executions = execute_if_between(
                comp,
                schedule.mappings["pe_sched"][r][c].i1,
                schedule.mappings["pe_sched"][r][c].i2,
                [get_pe_invoke(r, c, pe_accum_cond)],
            )
            output_writes = execute_if_eq(
                comp,
                schedule.mappings["pe_write_sched"][r][c].i1,
                [py_ast.Enable(NAME_SCHEME["out write"].format(pe=f"pe_{r}_{c}"))],
            )
            while_body_stmts.append(
                py_ast.StaticParComp(input_mem_updates + pe_executions + output_writes)
            )
            # providing metadata
            # tag = counter()
    #             source_map[
    #                 tag
    #             ] = f"pe_{r}_{c} filling: [{schedules['fill_sched'][r][c][0]},\
    # {schedules['fill_sched'][r][c][1]}), \
    # accumulating: [{schedules['accum_sched'][r][c][0]} \
    # {schedules['accum_sched'][r][c][1]}), \
    # writing: [{schedules['write_sched'][r][c][0]} \
    # {schedules['write_sched'][r][c][1]})"

    while_body = py_ast.StaticParComp(while_body_stmts)

    # build the while loop with condition cond_reg.
    if config.static:
        while_loop = cb.static_repeat(config.get_iteration_count(), while_body)
    else:
        cond_reg_port = comp.get_cell("lt_iter_limit").port("out")
        while_loop = cb.while_(cond_reg_port, while_body)

    control.append(while_loop)

    if config.static:
        return py_ast.StaticSeqComp(stmts=control), source_map
    return py_ast.SeqComp(stmts=control), source_map


def create_systolic_array(prog: cb.Builder, config: SystolicConfiguration):
    """
    top_length: Number of PEs in each row.
    top_depth: Number of elements processed by each PE in a row.
    left_length: Number of PEs in each column.
    left_depth: Number of elements processed by each PE in a col.
    """
    pe(prog)
    computational_unit = prog.component(SYSTOLIC_ARRAY_COMP)
    depth_port = computational_unit.input("depth", BITWIDTH)
    # initialize the iteration limit to top_length + left_length + depth + 4
    init_iter_limit(computational_unit, depth_port, config)

    # Generate the Schedule
    schedule = gen_schedules(config, computational_unit)

    # instantiate groups that handles the idx variables
    instantiate_idx_groups(computational_unit, config)

    # Generate the hardware For the schedule
    schedule.build_hardware(computational_unit, computational_unit.get_cell("idx"))

    for row in range(config.left_length):
        for col in range(config.top_length):
            # Instantiate the PEs and surronding registers
            instantiate_pe(computational_unit, row, col)

    # Instantiate all the memories
    for r in range(config.top_length):
        instantiate_memory(computational_unit, "top", r, config.top_depth)

    for col in range(config.left_length):
        instantiate_memory(computational_unit, "left", col, config.left_depth)

    # Instantiate output memory
    for i in range(config.left_length):
        add_systolic_output_params(
            computational_unit, i, bits_needed(config.top_length)
        )

    # Instantiate all the PEs
    for row in range(config.left_length):
        for col in range(config.top_length):
            # Instantiate the mover fabric
            instantiate_data_move(
                computational_unit,
                row,
                col,
                col == config.top_length - 1,
                row == config.left_length - 1,
            )

            # Instantiate output movement structure, i.e., writes to
            # `computational_unit`'s output ports
            instantiate_output_move(computational_unit, row, col)

    # Generate the control and set the source map
    control, source_map = generate_control(computational_unit, config, schedule)
    computational_unit.control = control
    prog.program.meta = source_map
