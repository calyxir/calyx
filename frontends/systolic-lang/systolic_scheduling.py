import calyx.builder as cb
from gen_pe import BITWIDTH
from enum import Enum
from systolic_arg_parser import SystolicConfiguration
import numpy as np


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
        if type == ScheduleType.INTERVAL and self.i2 is None:
            raise Exception("INTERVAL type must specify beginning and end")

    def __lt__(self, other):
        return (self.type, self.i1, self.i2) < (other.type, other.i1, other.i2)


class Schedule:
    def __init__(self):
        # XXX(Caleb): self.instances could be a set, but I'm running into annoying
        # ordering errors on tests. Python dictionaries are luckily ordered.
        self.instances = {}
        self.mappings = {}

    def add_instances(self, name, schedule_instances):
        """ """
        self.mappings[name] = schedule_instances
        for schedule_instance in schedule_instances.flatten():
            self.instances[schedule_instance] = None

    def __instantiate_calyx_adds(self, comp) -> list:
        """ """
        for schedule_instance in self.instances.keys():
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
        # Dictionary to keep consistent ordering.
        ge_ranges = {}
        lt_ranges = {}
        eq_ranges = {}
        interval_ranges = {}
        for schedule_instance in self.instances.keys():
            sched_type = schedule_instance.type
            if sched_type == ScheduleType.GE:
                ge_ranges[schedule_instance.i1] = None
            elif sched_type == ScheduleType.LT:
                lt_ranges[schedule_instance.i1] = None
            elif sched_type == ScheduleType.EQ:
                eq_ranges[schedule_instance.i1] = None
            elif sched_type == ScheduleType.INTERVAL:
                ge_ranges[schedule_instance.i1] = None
                lt_ranges[schedule_instance.i2] = None
                interval_ranges[(schedule_instance.i1, schedule_instance.i2)] = None
        self.__instantiate_calyx_adds(comp)
        # Need to sort for testing purposes
        for val in eq_ranges:
            self.__check_idx_eq(comp, idx_reg, val)
        for val in ge_ranges:
            self.__check_idx_lower_bound(comp, idx_reg, val)
        for val in lt_ranges:
            self.__check_idx_upper_bound(comp, idx_reg, val)
        for start, end in interval_ranges:
            self.__check_idx_between(comp, start, end)


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
