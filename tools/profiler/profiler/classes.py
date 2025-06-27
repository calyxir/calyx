import os
import copy
import json
from collections import defaultdict, deque
from dataclasses import dataclass, field
from enum import Enum
import statistics

from profiler.errors import ProfilerException


@dataclass
class SourceLoc:
    """
    ADL source location information obtained from metadata.
    """

    filename: str
    linenum: int
    varname: str

    def __init__(self, json_dict):
        self.filename = os.path.basename(json_dict["filename"])
        self.linenum = json_dict["linenum"]
        self.varname = json_dict["varname"]

    def adl_str(self):
        return f"{self.varname} {{{self.filename}: {self.linenum}}}"

    def loc_str(self):
        return f"{{{self.filename}: {self.linenum}}}"


@dataclass
class AdlMap:
    """
    Mappings from Calyx components, cells, and groups to the corresponding ADL SourceLoc.
    """

    # component --> (filename, linenum)
    component_map: dict[str, SourceLoc]
    # component --> {cell --> (filename, linenum)}
    cell_map: dict[str, dict[str, SourceLoc]]
    # component --> {group --> (filename, linenum)}
    group_map: dict[str, dict[str, SourceLoc]]

    def __init__(self, adl_mapping_file: str):
        self.component_map = {}
        self.cell_map = {}
        self.group_map = {}
        with open(adl_mapping_file, "r") as json_file:
            json_data = json.load(json_file)
            for component_dict in json_data:
                component_name = component_dict["component"]
                self.component_map[component_name] = SourceLoc(component_dict)
                self.cell_map[component_name] = {}
                for cell_dict in component_dict["cells"]:
                    self.cell_map[component_name][cell_dict["name"]] = SourceLoc(
                        cell_dict
                    )
                # probably worth removing code clone at some point
                self.group_map[component_name] = {}
                for group_dict in component_dict["groups"]:
                    self.group_map[component_name][group_dict["name"]] = SourceLoc(
                        group_dict
                    )


@dataclass
class CellMetadata:
    """
    Preprocessed information related to cells.
    """

    main_component: str
    # component name --> [cell names]
    component_to_cells: dict[str, list[str]]
    # component name --> { old cell --> new cell}
    shared_cells: dict[str, dict[str, str]] = field(default_factory=dict)
    added_signal_prefix: bool = field(default=False)

    # optional fields to fill in later

    # OS-specific Verilator prefix
    signal_prefix: str | None = field(default=None)

    def add_signal_prefix(self, signal_prefix: str):
        """
        Add OS-specific Verilator prefix to all cell names
        """
        assert not self.added_signal_prefix
        self.signal_prefix = signal_prefix
        str_to_add = signal_prefix + "."
        self.main_component = str_to_add + self.main_component

        for component in self.component_to_cells:
            fq_cells = [
                f"{signal_prefix}.{cell}" for cell in self.component_to_cells[component]
            ]
            self.component_to_cells[component] = fq_cells

        self.added_signal_prefix = True

    def get_component_of_cell(self, cell: str):
        """
        Obtain the name of the component from which a cell comes from.
        """
        for component in self.component_to_cells:
            if cell in self.component_to_cells[component]:
                return component
        raise ProfilerException(
            f"Lookup of cell that doesn't have a corresponding component! Cell name: {cell}"
        )

    @property
    def cells(self) -> list[str]:
        cells = []
        for component in self.component_to_cells:
            cells += self.component_to_cells[component]
        return cells

    @property
    def main_shortname(self):
        # Name of the main component without the signal prefix
        suffix = self.main_component.rsplit(".", 1)[-1]
        return suffix


class ParChildType(Enum):
    GROUP = 1
    PAR = 2


@dataclass(frozen=True)
class ParChildInfo:
    child_name: str
    child_type: ParChildType
    parents: set[str] = field(default_factory=set)

    def register_new_parent(self, new_parent: str):
        # FIXME: deprecate this method and instead obtain the entire parent set upfront.
        self.parents.add(new_parent)


@dataclass
class ControlMetadata:
    """
    Preprocessed information on TDCC-generated FSMs and control groups (only pars so far).
    """

    # names of fully qualified FSMs
    fsms: set[str] = field(default_factory=set)
    # names of fully qualified par groups
    par_groups: set[str] = field(default_factory=set)
    # component --> { fsm in the component. NOT fully qualified }
    # components that are not in this dictionary do not contain any fsms
    component_to_fsms: defaultdict[str, set[str]] = field(
        default_factory=lambda: defaultdict(set)
    )
    # component --> { par groups in the component }
    # components that are not in this dictionary do not contain any par groups
    component_to_par_groups: defaultdict[str, set[str]] = field(
        default_factory=lambda: defaultdict(set)
    )
    # fully qualified par name --> [fully-qualified par children name]. Each of the children here have to be pars.
    par_to_par_children: defaultdict[str, list[str]] = field(
        default_factory=lambda: defaultdict(list)
    )
    # component --> { child name --> ParChildInfo (contains parent name(s) and child type) }
    component_to_child_to_par_parent: dict[str, dict[str, ParChildInfo]] = field(
        default_factory=dict
    )
    # fully qualified names of done registers for pars
    par_done_regs: set[str] = field(default_factory=set)
    # partial_fsm_events:

    cell_to_ordered_pars: defaultdict[str, list[str]] = field(
        default_factory=lambda: defaultdict(list)
    )  # cell --> ordered par group names

    added_signal_prefix: bool = field(default=False)

    def add_par_done_reg(self, par_done_reg):
        self.par_done_regs.add(par_done_reg)

    def register_fully_qualified_par(self, fully_qualified_par):
        self.par_groups.add(fully_qualified_par)

    def add_signal_prefix(self, signal_prefix: str):
        assert not self.added_signal_prefix
        self.fsms = {f"{signal_prefix}.{fsm}" for fsm in self.fsms}
        self.par_done_regs = {f"{signal_prefix}.{pd}" for pd in self.par_done_regs}
        self.par_groups = {
            f"{signal_prefix}.{par_group}" for par_group in self.par_groups
        }
        new_par_to_children = defaultdict(list)
        for fully_qualified_par in self.par_to_par_children:
            new_par_to_children[f"{signal_prefix}.{fully_qualified_par}"] = list(
                map(
                    lambda c: f"{signal_prefix}.{c}",
                    self.par_to_par_children[fully_qualified_par],
                )
            )
        self.par_to_par_children = new_par_to_children
        self.added_signal_prefix = True

    def register_fsm(self, fsm_name, component, cell_metadata: CellMetadata):
        """
        Add information about a newly discovered FSM to the fields fsms and component_to_fsms.
        """
        if component not in cell_metadata.component_to_cells:
            # skip FSMs from components listed in primitive files (not in user-defined code)
            return
        self.component_to_fsms[component].add(fsm_name)

        for cell in cell_metadata.component_to_cells[component]:
            fully_qualified_fsm = ".".join((cell, fsm_name))
            self.fsms.add(fully_qualified_fsm)

    def register_par(self, par_group, component):
        self.component_to_par_groups[component].add(par_group)

    def register_par_child(
        self,
        component: str,
        child_name: str,
        parent: str,
        child_type: ParChildType,
        cell_metadata: CellMetadata,
    ):
        """
        Add information about a par child to the fields component_to_child_to_par_parent and par_to_children.
        """
        if component in self.component_to_child_to_par_parent:
            if child_name in self.component_to_child_to_par_parent[component]:
                self.component_to_child_to_par_parent[component][
                    child_name
                ].register_new_parent(parent)
            else:
                child_info = ParChildInfo(child_name, child_type, {parent})
                self.component_to_child_to_par_parent[component][child_name] = (
                    child_info
                )
        else:
            child_info = ParChildInfo(child_name, child_type, {parent})
            self.component_to_child_to_par_parent[component] = {child_name: child_info}

        if child_type == ParChildType.PAR:
            for cell in cell_metadata.component_to_cells[component]:
                fully_qualified_par = f"{cell}.{parent}"
                fully_qualified_child = f"{cell}.{child_name}"

                self.par_to_par_children[fully_qualified_par].append(
                    fully_qualified_child
                )

    def order_pars(self, cell_metadata: CellMetadata):
        """
        Give a partial ordering for pars so we know when multiple pars occur simultaneously, what order
        we should add them to the trace.
        (1) order based on cells
        (2) for pars in the same cell, order based on dependencies information

        Updates: - field cell_to_ordered_pars
        """

        cells_to_pars_without_parent: dict[str, set[str]] = {}
        for component in self.component_to_par_groups:
            pars = self.component_to_par_groups[component]
            pars_with_parent = [
                k
                for k, v in self.component_to_child_to_par_parent[component].items()
                if v.child_type == ParChildType.PAR
            ]
            pars_without_parent = pars.difference(pars_with_parent)
            for cell in cell_metadata.component_to_cells[component]:
                cells_to_pars_without_parent[cell] = pars_without_parent

        for cell in sorted(
            cells_to_pars_without_parent.keys(), key=(lambda c: c.count("."))
        ):
            # worklist contains pars to check whether they have children
            # the worklist starts with pars with no parent
            worklist: deque[str] = deque(
                [f"{cell}.{par}" for par in cells_to_pars_without_parent[cell]]
            )

            while worklist:
                par = worklist.pop()
                if par not in self.cell_to_ordered_pars[cell]:
                    self.cell_to_ordered_pars[cell].append(par)
                if par in self.par_to_par_children:
                    # get all the children (who are pars) of this par.
                    # If this par is not in self.par_to_par_children, it means that it has no children who are pars.
                    worklist.extendleft(self.par_to_par_children[par])


class StackElementType(Enum):
    GROUP = 1
    PRIMITIVE = 2
    CELL = 3
    CONTROL_GROUP = 4  # TDCC-generated groups that manage control


@dataclass
class StackElement:
    """
    An element on a trace stack.
    """

    # the name of the element determined by the profiler process; may not be the original name of the entity
    internal_name: str
    element_type: StackElementType
    is_main: bool = field(default=False)

    # should only contain a value if element_type is CELL
    component_name: str | None = field(default=None)
    # should only contain a value if element_type is CELL
    replacement_cell_name: str | None = field(default=None)

    # ADL source location of the stack element
    sourceloc: SourceLoc | None = field(default=None)
    # ADL source location of the replacement cell
    # Should only contain a value if element_type is CELL
    replacement_cell_sourceloc: SourceLoc | None = field(default=None)
    # ADL source location of the original component definition
    # Should only contain a value if element_type is CELL
    component_sourceloc: SourceLoc | None = field(default=None)

    compiler_generated_msg = "compiler-generated"

    # suffix after control enable group name generated by the unique-control compiler pass
    unique_group_str = "UG"

    @property
    def name(self) -> str:
        if (
            self.element_type == StackElementType.GROUP
            and self.unique_group_str in self.internal_name
        ):
            # control enabled group given a unique identifier name
            return self.internal_name.split(self.unique_group_str)[0]
        else:
            return self.internal_name

    def __repr__(self):
        match self.element_type:
            case StackElementType.GROUP:
                return self.name
            case StackElementType.PRIMITIVE:
                return f"{self.name} (primitive)"
            case StackElementType.CELL:
                if self.is_main:
                    return f"{self.name}"
                elif self.replacement_cell_name is not None:
                    return f"{self.name} ({self.replacement_cell_name}) [{self.component_name}]"
                else:
                    return f"{self.name} [{self.component_name}]"
            case StackElementType.CONTROL_GROUP:
                return f"{self.name} (ctrl)"

    def adl_str(self):
        """
        String representation for ADL flame graph.
        Any name in '' (single quotes) indicates an entity created by the compiler (doesn't exist in the original ADL code).
        """
        match self.element_type:
            case StackElementType.GROUP:
                if self.sourceloc is None:
                    return f"'{self.name}' {{{self.compiler_generated_msg}}}"
                else:
                    return self.sourceloc.adl_str()
            case StackElementType.PRIMITIVE:
                return f"{self.sourceloc.adl_str()} (primitive)"
            case StackElementType.CELL:
                if self.is_main:
                    return self.sourceloc.adl_str()
                else:
                    og_sourceloc_str = self.sourceloc.adl_str()
                    component_sourceloc_str = self.component_sourceloc.adl_str()
                    if self.replacement_cell_name is not None:
                        replacement_sourceloc_str = (
                            self.replacement_cell_sourceloc.adl_str()
                        )
                        return f"{og_sourceloc_str} ({replacement_sourceloc_str}) [{component_sourceloc_str}]"
                    else:
                        return f"{og_sourceloc_str} [{component_sourceloc_str}]"
            case StackElementType.CONTROL_GROUP:
                return f"{self.compiler_generated_msg} (ctrl)"

    def mixed_str(self):
        """
        String representation for mixed (group/cell/component names in Calyx, along with sourceloc file and line #) flame graph.
        """
        match self.element_type:
            case StackElementType.GROUP:
                if self.sourceloc is None:
                    return f"{self.name} {{{self.compiler_generated_msg}}}"
                else:
                    return f"{self.name} {self.sourceloc.loc_str()}"
            case StackElementType.PRIMITIVE:
                return f"{self.name} (primitive) {self.sourceloc.loc_str()}"
            case StackElementType.CELL:
                if self.is_main:
                    return f"{self.name} {self.sourceloc.loc_str()}"
                else:
                    og_str = f"{self.name} {self.sourceloc.loc_str()}"
                    component_str = (
                        f"{self.component_name} {self.component_sourceloc.loc_str()}"
                    )
                    if self.replacement_cell_name is not None:
                        replacement_str = self.replacement_cell_sourceloc.loc_str()
                        return f"{og_str} ({replacement_str}) [{component_str}]"
                    else:
                        return f"{og_str} [{component_str}]"
            case StackElementType.CONTROL_GROUP:
                return f"{self.name} (ctrl) {{{self.compiler_generated_msg}}}"


class FlameMapMode(Enum):
    CALYX = 1
    ADL = 2
    MIXED = 3


class CycleType(Enum):
    GROUP_OR_PRIMITIVE = 1  # at least one group/primitive is executing this cycle
    FSM_UPDATE = 2  # only fsm updates are happening this cycle
    PD_UPDATE = 3  # only pd register updates are happening this cycle
    MULT_CONTROL = 4  # fsm and par-done
    OTHER = 5  # most likely a compiler-generated group


class CycleTrace:
    """
    List of stacks that are active in a single cycle.
    """

    stacks: list[list[StackElement]]
    is_useful_cycle: bool

    sourceloc_info_added: bool = field(default=False)

    def __repr__(self):
        out = ""
        out = "\n".join(map(lambda x: f"\t{x}", self.stacks))
        return out

    def __init__(self, stacks_this_cycle: list[list[StackElement]] | None = None):
        self.stacks = []

        # If a group or primitive is at the top of at least one stack, then the cycle is "useful"
        self.is_useful_cycle = False
        if stacks_this_cycle is not None:
            for stack in stacks_this_cycle:
                self.add_stack(stack)

    def add_stack(self, stack: list[StackElement]):
        assert len(stack) > 0
        top: StackElement = stack[-1]
        match top.element_type:
            case StackElementType.GROUP | StackElementType.PRIMITIVE:
                self.is_useful_cycle = True
                # self.cycle_type = CycleType.GROUP_OR_PRIMITIVE
        self.stacks.append(stack)

    def get_stack_str_list(self, mode: FlameMapMode):
        """
        Retrieve a list of stack string representations based on what mode (Default, ADL, mixed) we're going off of.
        """
        stack_str_list = []
        for stack in self.stacks:
            match mode:
                case FlameMapMode.CALYX:
                    stack_str = ";".join(map(lambda elem: str(elem), stack))
                case FlameMapMode.ADL:
                    assert self.sourceloc_info_added
                    stack_str = ";".join(map(lambda elem: elem.adl_str(), stack))
                case FlameMapMode.MIXED:
                    assert self.sourceloc_info_added
                    stack_str = ";".join(map(lambda elem: elem.mixed_str(), stack))
            stack_str_list.append(stack_str)
        return stack_str_list

    def get_num_stacks(self):
        return len(self.stacks)

    def add_sourceloc_info(self, adl_map: AdlMap):
        """
        Adds ADL mapping information to elements on stacks. Elements that don't get ADL information added will be considered compiler-generated.
        """
        # FIXME: Need to consider how the new unique-control pass would affect this function.
        # Maybe we use `stack_elem.internal_name` instead? I'm not 100% sure.
        for stack in self.stacks:
            curr_component: str | None = None
            for stack_elem in stack:
                match stack_elem.element_type:
                    case StackElementType.CELL:
                        if stack_elem.is_main:
                            stack_elem.sourceloc = adl_map.component_map[
                                stack_elem.name
                            ]
                            curr_component = stack_elem.name
                        else:
                            stack_elem.sourceloc = adl_map.cell_map[curr_component][
                                stack_elem.name
                            ]
                            cell_component = stack_elem.component_name
                            stack_elem.component_sourceloc = adl_map.component_map[
                                cell_component
                            ]
                            if stack_elem.replacement_cell_name is not None:
                                stack_elem.replacement_cell_sourceloc = (
                                    adl_map.cell_map[curr_component][
                                        stack_elem.replacement_cell_name
                                    ]
                                )
                            curr_component = cell_component
                    case StackElementType.GROUP:
                        # compiler-generated groups will not be contained in adl_map.group_map
                        if (
                            stack_elem.internal_name
                            in adl_map.group_map[curr_component]
                        ):
                            stack_elem.sourceloc = adl_map.group_map[curr_component][
                                stack_elem.internal_name
                            ]
                    case StackElementType.PRIMITIVE:
                        stack_elem.sourceloc = adl_map.cell_map[curr_component][
                            stack_elem.internal_name
                        ]

        self.sourceloc_info_added = True


class UtilizationCycleTrace(CycleTrace):
    """
    List of stacks that are active in a single cycle, containing utilization information
    (both aggregated and per primitive).
    """

    # Reference to the global utilization map from all primitives to their utilization
    global_utilization: dict[str, dict]
    # Aggregated utilization of all the primitives in this cycle
    # Ex. {'Total LUTs': 21, 'Logic LUTs': 5, 'LUTRAMs': 16, 'SRLs': 0, 'FFs': 38, 'RAMB36': 0, 'RAMB18': 0, 'URAM': 0, 'DSP Blocks': 0}
    utilization: dict
    # Map between primitives in this cycle and their utilization (subset of global_utilization filtered for this cycle)
    utilization_per_primitive: dict[str, dict]
    # List of all the primitives active in this cycle
    primitives_active: list[str]

    def __init__(
        self,
        utilization: dict[str, dict],
        stacks_this_cycle: list[list[StackElement]] | None = None,
    ):
        self.global_utilization = utilization
        self.utilization = {}
        self.primitives_active = []
        self.utilization_per_primitive = {}
        super().__init__(stacks_this_cycle)

    def __repr__(self):
        return super().__repr__() + f"\n\t{self.utilization}"

    def add_stack(self, stack):
        super().add_stack(stack)
        top: StackElement = stack[-1]
        if top.element_type == StackElementType.PRIMITIVE:
            primitive = ".".join(
                map(
                    lambda x: x.name,
                    filter(
                        lambda x: x.element_type == StackElementType.CELL or x == top,
                        stack,
                    ),
                )
            )
            self.primitives_active.append(primitive)
            for k, v in self.global_utilization.get(primitive, {}).items():
                if v.isdigit():
                    self.utilization[k] = self.utilization.get(k, 0) + int(v)
            self.utilization_per_primitive[primitive] = self.global_utilization.get(
                primitive, {}
            )


@dataclass
class GroupSummary:
    """
    Summary for groups on the number of times they were active vs their active cycles
    """

    display_name: str
    num_times_active: int = 0
    active_cycles: set[int] = field(default_factory=set)

    interval_lengths: list[int] = field(default_factory=list)

    def register_interval(self, interval: range):
        self.num_times_active += 1
        self.active_cycles.update(set(interval))
        self.interval_lengths.append(len(interval))

    def fieldnames():
        return [
            "group-name",
            "num-times-active",
            "total-cycles",
            "min",
            "max",
            "avg",
            "can-static",
        ]

    def stats(self):
        stats = {}
        stats["group-name"] = self.display_name
        stats["num-times-active"] = self.num_times_active
        stats["total-cycles"] = len(self.active_cycles)
        min_interval = min(self.interval_lengths)
        max_interval = max(self.interval_lengths)
        avg_interval = round(statistics.mean(self.interval_lengths), 1)
        stats["min"] = min_interval
        stats["max"] = max_interval
        stats["avg"] = avg_interval
        stats["can-static"] = "Y" if min_interval == max_interval else "N"
        return stats


@dataclass
class Summary:
    """
    Summary for Cells/Control groups on the number of times they were active vs their active cycles
    FIXME: Add min/max/avg and collect these for normal groups as well?
    """

    num_times_active: int = 0
    active_cycles: set[int] = field(default_factory=set)


class ControlRegUpdateType(Enum):
    FSM = 1
    PAR_DONE = 2
    BOTH = 3


@dataclass(frozen=True)
class ControlRegUpdates:
    """
    Updates to control registers in a cell.
    Retain this info to add to the timeline
    """

    cell_name: str
    clock_cycle: int
    updates: str


@dataclass
class TraceData:
    trace: dict[int, CycleTrace] = field(default_factory=dict)
    trace_classified: dict[int, CycleType] = field(default_factory=dict)
    cell_to_active_cycles: dict[str, Summary] = field(default_factory=dict)
    # primitive to active cycles?

    # fields relating to control groups/registers
    trace_with_control_groups: dict[int, CycleTrace] = field(default_factory=dict)
    control_group_to_active_cycles: dict[str, Summary] = field(default_factory=dict)
    # cell --> ControlRegUpdate. This is for constructing timeline later.
    control_reg_updates: dict[str, list[ControlRegUpdates]] = field(
        default_factory=dict
    )

    cycletype_to_cycles: dict[CycleType, set[int]] | None = None

    def print_trace(self, threshold=-1, ctrl_trace=False):
        """
        Threshold is an optional argument that determines how many cycles you are going to print out.
        """
        if threshold == 0:
            return
        trace = self.trace_with_control_groups if ctrl_trace else self.trace
        for i in trace:
            if 0 < threshold < i:
                print(f"\n... (total {len(self.trace)} cycles)")
                return
            print(i)
            print(trace[i])

    @staticmethod
    def incr_num_times_active(name: str, d: dict[str, Summary]):
        if name not in d:
            d[name] = Summary()
        d[name].num_times_active += 1

    def cell_start_invoke(self, cell: str):
        self.incr_num_times_active(cell, self.cell_to_active_cycles)

    def register_cell_cycle(self, cell, cycle: int):
        self.cell_to_active_cycles[cell].active_cycles.add(cycle)

    def control_group_interval(self, group: str, interval: range):
        self.incr_num_times_active(group, self.control_group_to_active_cycles)
        self.control_group_to_active_cycles[group].active_cycles.update(set(interval))

    def register_control_reg_update(self, cell: str, clock_cycle: int, update_str: str):
        if cell not in self.control_reg_updates:
            self.control_reg_updates[cell] = []
        self.control_reg_updates[cell].append(
            ControlRegUpdates(cell, clock_cycle, update_str)
        )

    def create_trace_with_control_groups(
        self,
        control_groups_trace: dict[int, set[str]],
        cell_metadata: CellMetadata,
        control_metadata: ControlMetadata,
        utilization: dict[str, dict] | None = None,
    ):
        """
        Populates the field trace_with_control_groups by combining control group information (from control_groups_trace) with self.trace.
        Does not modify self.trace.
        """
        control_metadata.order_pars(cell_metadata)
        for i in self.trace:
            if i in control_groups_trace:
                self.trace_with_control_groups[i] = (
                    CycleTrace()
                    if utilization is None
                    else UtilizationCycleTrace(utilization)
                )
                for events_stack in self.trace[i].stacks:
                    new_events_stack = self._create_events_stack_with_control_groups(
                        events_stack,
                        cell_metadata,
                        control_metadata,
                        control_groups_trace[i],
                    )
                    self.trace_with_control_groups[i].add_stack(new_events_stack)
            else:
                self.trace_with_control_groups[i] = copy.copy(self.trace[i])

    def _create_events_stack_with_control_groups(
        self,
        events_stack: list[StackElement],
        cell_metadata: CellMetadata,
        control_metadata: ControlMetadata,
        active_control_groups: set[str],
    ):
        """
        Helper method for create_trace_with_control_groups(). Returns new StackElement list that contain active control groups.
        """
        events_stack_with_ctrl: list[StackElement] = []
        for stack_element in events_stack:
            events_stack_with_ctrl.append(stack_element)
            match stack_element.element_type:
                case StackElementType.CELL:
                    if stack_element.is_main:
                        current_cell = (
                            f"{cell_metadata.signal_prefix}.{stack_element.name}"
                        )
                    else:
                        current_cell += f".{stack_element.internal_name}"
                case StackElementType.GROUP:
                    # standard groups to handle edge case of nested pars concurrent with groups; pop any pars that aren't this group's parent
                    current_component = cell_metadata.get_component_of_cell(
                        current_cell
                    )
                    if (
                        current_component
                        in control_metadata.component_to_child_to_par_parent
                        and stack_element.internal_name
                        in control_metadata.component_to_child_to_par_parent[
                            current_component
                        ]
                    ):
                        child_parent_info: ParChildInfo = (
                            control_metadata.component_to_child_to_par_parent[
                                current_component
                            ][stack_element.internal_name]
                        )
                        parent_names = set()
                        if child_parent_info.child_type == ParChildType.PAR:
                            raise ProfilerException(
                                "A normal group should not be stored as a par group under control_metadata.component_to_child_to_par_parent"
                            )
                        parent_names.update(child_parent_info.parents)
                        parent_found = False
                        while (
                            len(events_stack_with_ctrl) > 2
                            and events_stack_with_ctrl[-2].element_type
                            == StackElementType.CONTROL_GROUP
                        ):
                            # FIXME: We currently assume that all StackElementType.CONTROL_GROUP are pars, so we can pull this trick
                            # NOTE: we may need to fix this in the future when there are multiple StackElementType.CONTROL_GROUP
                            for parent in parent_names:
                                if parent == events_stack_with_ctrl[-2].internal_name:
                                    parent_found = True
                                    break
                            if parent_found:
                                break
                            events_stack_with_ctrl.pop(-2)
                        continue
                    continue
                case StackElementType.PRIMITIVE:
                    # All primitives are leaf nodes, so there is no more work left to be done.
                    break
            if current_cell in control_metadata.cell_to_ordered_pars:
                active_from_cell = active_control_groups.intersection(
                    control_metadata.cell_to_ordered_pars[current_cell]
                )
                for par_group_active in sorted(
                    active_from_cell,
                    key=(
                        lambda p: control_metadata.cell_to_ordered_pars[
                            current_cell
                        ].index(p)
                    ),
                ):
                    par_group_name = par_group_active.split(".")[-1]
                    events_stack_with_ctrl.append(
                        StackElement(par_group_name, StackElementType.CONTROL_GROUP)
                    )
        return events_stack_with_ctrl

    def add_sourceloc_info(self, adl_map: AdlMap):
        """
        Wrapper function to add SourceLoc info to elements in self.trace
        """
        trace: dict[int, CycleTrace] = self.trace_with_control_groups
        assert len(trace) > 0  # can't add sourceloc info on an empty trace

        for i in self.trace:
            trace[i].add_sourceloc_info(adl_map)

        return trace
