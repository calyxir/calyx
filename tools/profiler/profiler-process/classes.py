import os
import copy
import json
from collections import defaultdict, deque
from dataclasses import dataclass, field
from enum import Enum

from errors import ProfilerException


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
    # cell name --> component name
    cell_to_component: dict[str, str]
    # component name --> [cell names]
    component_to_cells: dict[str, list[str]]
    # component name --> { old cell --> new cell}
    shared_cells: dict[str, dict[str, str]] = field(default_factory=dict)
    added_signal_prefix: bool = field(default=False)

    # optional fields to fill in later

    # Name of the main component without the signal prefix
    main_shortname: str | None = field(default=None)
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

        fq_cells = {f'{signal_prefix}.{cell}': comp for cell, comp in self.cell_to_component.items()}
        self.cell_to_component = fq_cells

        for component in self.component_to_cells:
            fully_qualified_cells = []
            for cell in self.component_to_cells[component]:
                fully_qualified_cells.append(str_to_add + cell)
            self.component_to_cells[component] = fully_qualified_cells

        self.added_signal_prefix = True

    def get_main_shortname(self):
        if self.main_shortname is None:
            self.main_shortname = self.main_component.split(".")[-1]
        return self.main_shortname


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
    component_to_fsms: defaultdict[str, set[str]] = field(default_factory=lambda: defaultdict(set))
    # component --> { par groups in the component }
    # components that are not in this dictionary do not contain any par groups
    component_to_par_groups: defaultdict[str, set[str]] = field(default_factory=lambda: defaultdict(set))
    # fully qualified par name --> [fully-qualified par children name]. Each of the children here have to be pars.
    par_to_par_children: defaultdict[str, list[str]] = field(default_factory=lambda: defaultdict(list))
    # component --> { child name --> ParChildInfo (contains parent name(s) and child type) }
    component_to_child_to_par_parent: dict[str, dict[str, ParChildInfo]] = field(
        default_factory=dict
    )
    # fully qualified names of done registers for pars
    par_done_regs: set[str] = field(default_factory=set)
    # partial_fsm_events:

    cell_to_ordered_pars: defaultdict[str, list[str]] = field(default_factory=lambda: defaultdict(list))  # cell --> ordered par group names

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

                self.par_to_par_children[fully_qualified_par].append(fully_qualified_child)

    def order_pars(self, cell_metadata: CellMetadata):
        """
        Give a partial ordering for pars so we know when multiple pars occur simultaneously, what order
        we should add them to the trace.
        (1) order based on cells
        (2) for pars in the same cell, order based on dependencies information
        """
        for cell in sorted(
            cell_metadata.cell_to_component.keys(), key=(lambda c: c.count("."))
        ):
            component = cell_metadata.cell_to_component[cell]
            if component not in self.component_to_par_groups:
                # ignore components that don't feature pars.
                continue
            pars = self.component_to_par_groups[component]
            # the worklist starts with pars with no parent
            pars_with_parent = [k for k, v in self.component_to_child_to_par_parent[component].items() if v.child_type == ParChildType.PAR]

            # need to make all of the pars fully qualified before adding them to the worklist.
            worklist: deque = deque([f"{cell}.{par}" for par in pars.difference(pars_with_parent)])

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

    name: str
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
                        if stack_elem.name in adl_map.group_map[curr_component]:
                            stack_elem.sourceloc = adl_map.group_map[curr_component][
                                stack_elem.name
                            ]
                    case StackElementType.PRIMITIVE:
                        stack_elem.sourceloc = adl_map.cell_map[curr_component][
                            stack_elem.name
                        ]

        self.sourceloc_info_added = True


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


@dataclass
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

    def incr_num_times_active(self, name: str, d: dict[str, Summary]):
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
    ):
        control_metadata.order_pars(cell_metadata)
        for i in self.trace:
            if i in control_groups_trace:
                self.trace_with_control_groups[i] = CycleTrace()
                for events_stack in self.trace[i].stacks:
                    new_events_stack: list[StackElement] = []
                    for stack_element in events_stack:
                        new_events_stack.append(stack_element)
                        match stack_element.element_type:
                            case StackElementType.CELL:
                                if stack_element.is_main:
                                    current_cell = f"{cell_metadata.signal_prefix}.{stack_element.name}"
                                else:
                                    current_cell += f".{stack_element.name}"
                            case StackElementType.GROUP:
                                # standard groups to handle edge case of nested pars concurrent with groups; pop any pars that aren't this group's parent
                                current_component = cell_metadata.cell_to_component[
                                    current_cell
                                ]
                                if (
                                    current_component
                                    in control_metadata.component_to_child_to_par_parent
                                    and stack_element.name
                                    in control_metadata.component_to_child_to_par_parent[
                                        current_component
                                    ]
                                ):
                                    child_parent_info: ParChildInfo = control_metadata.component_to_child_to_par_parent[
                                        current_component
                                    ][stack_element.name]
                                    parent_names = set()
                                    if (
                                            child_parent_info.child_type
                                            == ParChildType.PAR
                                        ):
                                            raise ProfilerException(
                                                "A normal group should not be stored as a par group under control_metadata.component_to_child_to_par_parent"
                                            )
                                    parent_names.update(child_parent_info.parents)
                                    parent_found = False
                                    while (
                                        len(new_events_stack) > 2
                                        and new_events_stack[-2].element_type
                                        == StackElementType.CONTROL_GROUP
                                    ):
                                        # FIXME: We currently assume that all StackElementType.CONTROL_GROUP are pars, so we can pull this trick
                                        # NOTE: we may need to fix this in the future when there are multiple StackElementType.CONTROL_GROUP
                                        for parent in parent_names:
                                            if parent == new_events_stack[-2].name:
                                                parent_found = True
                                                break
                                        if parent_found:
                                            break
                                        new_events_stack.pop(-2)
                                    continue
                                continue
                            case StackElementType.PRIMITIVE:
                                continue
                        if current_cell in control_metadata.cell_to_ordered_pars:
                            active_from_cell = control_groups_trace[i].intersection(
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
                                new_events_stack.append(
                                    StackElement(
                                        par_group_name, StackElementType.CONTROL_GROUP
                                    )
                                )
                    self.trace_with_control_groups[i].add_stack(new_events_stack)
            else:
                self.trace_with_control_groups[i] = copy.copy(self.trace[i])

    def add_sourceloc_info(self, adl_map: AdlMap):
        """
        Wrapper function to add SourceLoc info to elements in self.trace
        """
        trace: dict[int, CycleTrace] = self.trace_with_control_groups
        assert len(trace) > 0  # can't add sourceloc info on an empty trace

        for i in self.trace:
            trace[i].add_sourceloc_info(adl_map)

        return trace
