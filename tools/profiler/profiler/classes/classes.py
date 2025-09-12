import os
import copy
import json
from collections import defaultdict
from dataclasses import dataclass, field
from enum import Enum
import statistics

from profiler.errors import ProfilerException

# hello

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
class Utilization:
    """
    Hierarchical utilization wrapper.
    """

    map: dict[str, dict[str, str]]
    accessed: set[str]

    def __init__(self, json_dict):
        self.map = json_dict
        self.accessed = set()

    def get_module(self, name: str) -> dict[str, str]:
        """
        Get the utilization map for a module. `name` is a fully qualified name
        of a module on a stack.
        """
        if name in self.map:
            self.accessed.add(name)
        return self.map.get(name, {})

    def get_unaccessed(self):
        """
        Get a set of unaccessed modules in the utilization map.
        """
        module_set = set(k for k in self.map)
        return module_set.difference(self.accessed)


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
    ctrl_groups: set[str] = field(default_factory=set)
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
    # fully qualified names of done registers for pars
    par_done_regs: set[str] = field(default_factory=set)
    # component name --> { control group name --> { primitives used by control group } }
    component_to_control_to_primitives: defaultdict[str, defaultdict[str, set[str]]] = (
        field(default_factory=lambda: defaultdict(lambda: defaultdict(set)))
    )

    cell_to_tdcc_groups: defaultdict[str, set[str]] = field(
        default_factory=lambda: defaultdict(set)
    )  # cell --> { tdcc groups }

    component_to_ctrl_group_to_pos_str: (
        defaultdict[str, defaultdict[str, str]] | None
    ) = None

    # Store enable to path descriptor for each component
    component_to_enable_to_desc: dict[str, dict[str, str]] = field(default_factory=dict)

    # Store control statements' descriptors to
    component_to_ctrl_group_to_desc: dict[str, dict[str, int]] = field(
        default_factory=dict
    )

    added_signal_prefix: bool = field(default=False)

    def add_par_done_reg(self, component, par_group, par_done_reg, stack):
        self.par_done_regs.add(stack)
        self.component_to_control_to_primitives[component][par_group].add(par_done_reg)

    def register_fully_qualified_ctrl_gp(self, fully_qualified_gp):
        self.ctrl_groups.add(fully_qualified_gp)

    def add_signal_prefix(self, signal_prefix: str):
        assert not self.added_signal_prefix
        self.fsms = {f"{signal_prefix}.{fsm}" for fsm in self.fsms}
        self.par_done_regs = {f"{signal_prefix}.{pd}" for pd in self.par_done_regs}
        self.ctrl_groups = {
            f"{signal_prefix}.{ctrl_group}" for ctrl_group in self.ctrl_groups
        }
        self.added_signal_prefix = True
        fully_qualified_tdccs = {
            f"{signal_prefix}.{c}": g for c, g in self.cell_to_tdcc_groups.items()
        }
        self.cell_to_tdcc_groups = fully_qualified_tdccs

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

    def register_par(self, par_group: str, component: str):
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

    # Calyx source location of the control node.
    # Should only contain a value if element_type is CONTROL
    ctrl_loc_str: str | None = field(default=None)

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
                return (
                    f"{self.name} (primitive)"
                    if self.replacement_cell_name is None
                    else f"{self.name} (primitive) -> {self.replacement_cell_name}"
                )
            case StackElementType.CELL:
                if self.is_main:
                    return f"{self.name}"
                elif self.replacement_cell_name is not None:
                    return f"{self.name} ({self.replacement_cell_name}) [{self.component_name}]"
                else:
                    return f"{self.name} [{self.component_name}]"
            case StackElementType.CONTROL_GROUP:
                ctrl_string = (
                    f" ~ {self.ctrl_loc_str}" if self.ctrl_loc_str is not None else ""
                )
                return f"{self.name}{ctrl_string} (ctrl)"

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

    def add_stack(self, stack: list[StackElement], main_shortname: str = "main"):
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
    global_utilization: Utilization
    # Aggregated utilization of all the primitives in this cycle
    # Ex. {'Total LUTs': 21, 'Logic LUTs': 5, 'LUTRAMs': 16, 'SRLs': 0, 'FFs': 38, 'RAMB36': 0, 'RAMB18': 0, 'URAM': 0, 'DSP Blocks': 0}
    utilization: dict
    # Map between primitives in this cycle and their utilization (subset of global_utilization filtered for this cycle)
    utilization_per_primitive: dict[str, dict]
    # List of all the NON-CONTROL GROUP primitives active in this cycle
    primitives_active: set[str]
    # Reference to the control metadata, used for checking control groups
    control_metadata: ControlMetadata

    def __init__(
        self,
        utilization: Utilization,
        control_metadata: ControlMetadata,
        stacks_this_cycle: list[list[StackElement]] | None = None,
    ):
        self.global_utilization = utilization
        self.utilization = {}
        self.primitives_active = set()
        self.utilization_per_primitive = {}
        self.control_groups_active = []
        self.control_metadata = control_metadata
        super().__init__(stacks_this_cycle)

    def __repr__(self):
        return (
            super().__repr__()
            + f"\n\tUTIL: {', '.join(f'{k}: {v}' for k, v in self.utilization.items())}"
        )

    def add_stack(self, stack, main_shortname="main"):
        super().add_stack(stack)
        top: StackElement = stack[-1]
        fully_qualified_name = self._get_fully_qualified_name(stack)
        # if primitive (but not control primitive) then add directly to primitives_active
        if (
            top.element_type == StackElementType.PRIMITIVE
            and top.name not in self._flatten_control_primitives(top.component_name)
        ):
            self.primitives_active.add(fully_qualified_name)
        # if there are any control groups we call helper
        if any(e.element_type == StackElementType.CONTROL_GROUP for e in stack):
            self._add_control_group_utilization(stack, main_shortname)
        # get primitives utilization from global utilization map.
        # the little trick here is that this skips the control primitives since
        # those are processed separately. note that self.primitives_active does
        # NOT include control primitives!
        for p in self.primitives_active:
            util = {
                k: int(v) if v.isdigit() else v
                for k, v in self.global_utilization.get_module(p).items()
            }
            self.utilization_per_primitive[p] = util
        # populate aggregated cycle utilization
        self.utilization = {}
        for util in self.utilization_per_primitive.values():
            for k, v in util.items():
                if isinstance(v, int):
                    self.utilization[k] = self.utilization.get(k, 0) + v

    def _get_fully_qualified_name(self, stack: list[StackElement]):
        """
        Get the fully qualified name of a stack.
        """
        return ".".join(
            x.replacement_cell_name
            if x.replacement_cell_name
            else x.name  # we always replace cell-shared names if they exist
            for x in stack
            if x.element_type in {StackElementType.CELL, StackElementType.PRIMITIVE}
        )

    def _flatten_control_primitives(self, component: str):
        """
        Get control primitives from a component.
        """
        return {
            primitive
            for control_map in self.control_metadata.component_to_control_to_primitives.get(
                component, {}
            ).values()
            for primitive in control_map
        }

    def _add_control_group_utilization(
        self, stack: list[StackElement], main_shortname: str = "main"
    ):
        """
        Add utilization of primitives in control groups on stack to utilization per primitive.
        """
        stack_string = ""
        comp = main_shortname
        seen_groups = set()
        # accummulate control groups seen on the stack, with their fully qualified name
        # up until that point and the component they are in.
        for e in stack:
            if e.element_type == StackElementType.CONTROL_GROUP:
                seen_groups.add((stack_string, comp, e.name))
            if e.element_type == StackElementType.CELL:
                stack_string += f"{e.name}."
            if e.component_name:
                comp = e.component_name
        # get primitives used by each control group from the control metadata and
        # fetch their utilization from the global utilization map.
        # NOTE: we aggregate them based on control groups, because the user doesn't
        # really care about each individual par done register, but rather the combined
        # utilization of that control flow construct
        for prefix, comp, gp in seen_groups:
            key = f"{prefix}{gp}"
            primitives = self.control_metadata.component_to_control_to_primitives[comp][
                gp
            ]
            control_prims = {f"{prefix}{p}" for p in primitives}
            self.utilization_per_primitive[key] = {}

            for prim in control_prims:
                for k, v in self.global_utilization.get_module(prim).items():
                    if v.isdigit():
                        self.utilization_per_primitive[key][k] = (
                            self.utilization_per_primitive[key].get(k, 0) + int(v)
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
class PTrace:
    """
    A trace. Maps cycle indices to the CycleTrace that represents the
    stacks active in that cycle.
    When iterating over a PTrace, the values returned are cycle numbers/indices.
    """

    trace: list[CycleTrace] = field(default_factory=list)
    iter_idx: int = field(default=0)

    def add_cycle(self, i: int, cycle_trace: CycleTrace):
        """
        Adds an entry at cycle i to cycle_trace. If i is greater than the current number of cycles,
        adds blank CycleTraces between the current length and i.

        Invariant: i is not an existing cycle entry in the trace.
        """
        assert i >= len(self.trace)
        # padding with empty cycle traces, if there exists a gap
        while i > len(self.trace):
            self.trace.append(CycleTrace())
        self.trace.append(cycle_trace)

    def __getitem__(self, index):
        assert index <= len(self.trace)
        return self.trace[index]

    def __contains__(self, key):
        return key in self.trace

    def __iter__(self):
        self.iter_idx = 0
        return self

    def __next__(self):
        if self.iter_idx >= len(self.trace):
            raise StopIteration
        ret = self.iter_idx
        self.iter_idx += 1
        return ret

    def __len__(self):
        return len(self.trace)


@dataclass
class TraceData:
    # Set of all primitives and cells with continuous assignments
    cont_assignments: set[str] = field(default_factory=set)
    trace: PTrace = field(default_factory=PTrace)
    trace_classified: dict[int, CycleType] = field(default_factory=dict)
    cell_to_active_cycles: dict[str, Summary] = field(default_factory=dict)
    # primitive to active cycles?

    # fields relating to control groups/registers
    trace_with_control_groups: PTrace = field(default_factory=PTrace)
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
        if self.cont_assignments:
            print(f"\nCONT\t{', '.join(self.cont_assignments)}\n")

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
        utilization: Utilization | None = None,
    ):
        """
        Populates the field trace_with_control_groups by combining control group information (from control_groups_trace) with self.trace.
        Does not modify self.trace.
        """
        ctrl_groups_without_descriptor: set[str] = set()
        for i in self.trace:
            if i in control_groups_trace:
                new_cycletrace = (
                    CycleTrace()
                    if utilization is None
                    else UtilizationCycleTrace(utilization, control_metadata)
                )
                # fully qualified control group --> path descriptor
                active_control_group_to_desc: dict[str, str] = (
                    self._create_active_control_group_to_desc(
                        control_groups_trace[i],
                        cell_metadata,
                        control_metadata,
                        ctrl_groups_without_descriptor,
                    )
                )

                active_control_groups_missed: set[str] | None = None
                cell_to_stack_trace: dict[str, list[StackElement]] = {}
                for events_stack in self.trace[i].stacks:
                    stacks_to_add, missed_groups = (
                        self._add_events_stack_with_control_groups(
                            events_stack,
                            cell_metadata,
                            control_metadata,
                            active_control_group_to_desc,
                            cell_to_stack_trace,
                        )
                    )
                    # Add all control stacks
                    for stack in stacks_to_add:
                        new_cycletrace.add_stack(stack, cell_metadata.main_shortname)
                    if active_control_groups_missed is None:
                        # need to populate with the first set that gets returned
                        active_control_groups_missed = missed_groups
                    else:
                        active_control_groups_missed.intersection_update(missed_groups)
                # Edge case: add any control groups that weren't covered to the CycleTrace
                self._create_stacks_for_missed_control_groups(
                    active_control_groups_missed,
                    active_control_group_to_desc,
                    i,
                    cell_to_stack_trace,
                    cell_metadata,
                    control_metadata,
                )

                # add cycletrace to control groups trace
                self.trace_with_control_groups.add_cycle(i, new_cycletrace)

            else:
                self.trace_with_control_groups.add_cycle(i, copy.copy(self.trace[i]))

        for no_desc_group in sorted(ctrl_groups_without_descriptor):
            print(
                f"WARNING!!! No mapping from control group {no_desc_group} to a descriptor."
            )

    def _create_stacks_for_missed_control_groups(
        self,
        missed_groups: set[str],
        control_group_to_desc: dict[str, str],
        i: int,
        cell_to_stack_trace: dict[str, list[StackElement]],
        cell_metadata: CellMetadata,
        control_metadata: ControlMetadata,
    ):
        """
        Helper method to create_trace_with_control_groups() that handles any control groups that were active this cycle but not present
        in any created stacks. This can happen when there is a par block containing sequential blocks (a tdcc group inside of a par group)
        where the inner tdcc group is on a FSM register update cycle, but groups on the other par arms are active. New stacks are created
        to show any missing groups, which are added to the CycleTrace at cycle `i`.

        Assumption: A cell can only be invoked by a single group within a particular cycle. That is, _in a single cycle_, two groups cannot
        be the parent of a singular user-defined component cell. (This assumption is leveraged by the argument cell_to_stack_trace.)
        """
        if len(missed_groups) == 0:
            return

        active_control_group_to_parents: defaultdict[str, list[str]]
        leaf_control_groups: set[str]
        (active_control_group_to_parents, leaf_control_groups) = (
            self._compute_ctrl_group_to_parents(control_group_to_desc)
        )
        # control groups that weren't covered don't have a child group and were in parallel with a group in a different par arm
        # find leaves
        missed_leaves = missed_groups.intersection(leaf_control_groups)
        # create new stack for each leaf.
        for leaf_ctrl_group in missed_leaves:
            leaf_ctrl_group_split = leaf_ctrl_group.split(".")
            cell = ".".join(leaf_ctrl_group_split[:-1])
            cell_component = cell_metadata.get_component_of_cell(cell)
            leaf_name = leaf_ctrl_group_split[-1]
            new_stack: list[StackElement] = cell_to_stack_trace[cell].copy()
            # add parents of leaf
            for leaf_parent in active_control_group_to_parents[leaf_ctrl_group]:
                parent_stack_elem: StackElement = self._create_ctrl_stack_elem(
                    leaf_parent.split(".")[-1],
                    cell_component,
                    control_metadata,
                )
                new_stack.append(parent_stack_elem)
            # add leaf
            leaf_element: StackElement = self._create_ctrl_stack_elem(
                leaf_name, cell_component, control_metadata
            )
            new_stack.append(leaf_element)
            # add new_stack to the current cycle's CycleTrace
            self.trace_with_control_groups[i].add_stack(new_stack)

    def _compute_ctrl_group_to_parents(
        self, group_to_desc: dict[str, str]
    ) -> tuple[defaultdict[str, list[str]], set[str]]:
        """
        Helper function for _create_stacks_for_missed_control_groups() that returns:
         - a mapping from a fully qualified control group to a list of its ancestry (in order of oldest to newest) and the
         - the set of leaf control groups (groups that are not a parent of any other group), fully qualified

        ex) if the call order for the cell toplevel.main was tdcc0 --> par0 --> tdcc1, then the returned dict would look like:
        {
            "toplevel.main.tdcc0": [],
            "toplevel.main.tdcc1": ["toplevel.main.tdcc0", "toplevel.main.par0"],
            "toplevel.main.par0": ["toplevel.main.tdcc0"]
        }
        and the returned set would be a singleton: {"toplevel.main.tdcc1"}
        """
        # First, sort the control groups
        desc_to_group = {group_to_desc[k]: k for k in group_to_desc}
        ordered_groups = [desc_to_group[x] for x in sorted(desc_to_group.keys())]

        group_to_parents: defaultdict[str, list[str]] = defaultdict(list)
        # leaf_groups start with the set of all control groups, and elements are removed when they are found to be a parent
        leaf_groups: set[str] = set(group_to_desc.keys())
        for g in ordered_groups:
            g_desc = group_to_desc[g]
            for other_group in ordered_groups:
                other_desc = group_to_desc[other_group]
                if g != other_group and g_desc in other_desc:
                    # other_group is a child of g
                    group_to_parents[other_group].append(g)
                    # since g is a parent, remove from leaf_groups
                    leaf_groups.discard(g)

        return group_to_parents, leaf_groups

    def _create_active_control_group_to_desc(
        self,
        active_control_groups: set[str],
        cell_metadata: CellMetadata,
        control_metadata: ControlMetadata,
        groups_without_desc: set[str],
    ):
        """
        Helper function for create_trace_with_control_groups() that returns a mapping from
        fully qualified control groups to their path descriptors.
        """
        active_control_group_to_desc: dict[str, str] = {}
        for active_ctrl_group in active_control_groups:
            ctrl_group_split = active_ctrl_group.split(".")
            ctrl_group_cell = ".".join(ctrl_group_split[:-1])
            ctrl_group_name = ctrl_group_split[-1]
            ctrl_group_component = cell_metadata.get_component_of_cell(ctrl_group_cell)
            component_desc_map = control_metadata.component_to_ctrl_group_to_desc[
                ctrl_group_component
            ]
            if ctrl_group_name in component_desc_map:
                ctrl_group_desc = component_desc_map[ctrl_group_name]
                active_control_group_to_desc[active_ctrl_group] = ctrl_group_desc
            else:
                groups_without_desc.add(active_ctrl_group)

        return active_control_group_to_desc

    def _add_events_stack_with_control_groups(
        self,
        events_stack: list[StackElement],
        cell_metadata: CellMetadata,
        control_metadata: ControlMetadata,
        active_control_group_to_desc: dict[str, str],
        cell_to_stack_trace: dict[str, list[StackElement]],
    ) -> tuple[list[list[StackElement]], set[str]]:
        """
        Helper method for create_trace_with_control_groups().

        Returns:
          - a list of new StackElement list(s) that contain active control groups in order. If this cycle is a "useless cycle" containing
            parallel control groups as leaves, the size of this list would be greater than one.
          - the set of fully qualified control groups that were NOT included in the stack

        We determine the control groups to add on the stack from active_control_groups based on the path descriptors of each
        control group, and the path descriptor of the next non-control group (if there exists). All ancestoral control groups
        (any control group with a path descriptor that is a substring of the non-control group) get added in alphabetical order
        of path descriptor.

        Rules/Assumptions:
        - Control groups can only happen after cells
        - The next element after a cell in events_stack, if there exists one, is a group (verified by an assert)
        """
        events_stack_with_ctrl: list[StackElement] = []
        events_stacks_to_add: list[list[StackElement]] = []
        missed_control_groups: set[str] = set()
        for i in range(len(events_stack)):
            stack_element = events_stack[i]
            events_stack_with_ctrl.append(stack_element)
            match stack_element.element_type:
                case StackElementType.CELL:
                    # update the current cell.
                    if stack_element.is_main:
                        current_cell = (
                            f"{cell_metadata.signal_prefix}.{stack_element.name}"
                        )
                    else:
                        current_cell += f".{stack_element.internal_name}"

                    # add the current cell to cell_to_stack_trace if it doesn't already exist
                    if current_cell not in cell_to_stack_trace:
                        cell_to_stack_trace[current_cell] = (
                            events_stack_with_ctrl.copy()
                        )

                    # need to figure out which control groups are getting added to this cell
                    cell_component = cell_metadata.get_component_of_cell(current_cell)

                    # NOTE: make this part into a helper function?
                    # descriptor --> control group (NOT fully qualified)
                    active_ctrl_desc_to_group: dict[str, str] = {
                        active_control_group_to_desc[x]: x.split(".")[-1]
                        for x in active_control_group_to_desc
                        if ".".join(x.split(".")[:-1]) == current_cell
                    }

                    # Determine the control groups to add and their ordering.
                    # To do so, we must first figure out what is the next element in the stack, if there exists any
                    if i < len(events_stack) - 1:
                        # check the next element (has to be a non-structurally enabled group)
                        next_elem = events_stack[i + 1]
                        assert next_elem.element_type == StackElementType.GROUP
                        next_elem_descriptor = (
                            control_metadata.component_to_enable_to_desc[
                                cell_component
                            ][next_elem.internal_name]
                        )

                        # sort by whether we should add this
                        ctrl_groups_to_add = []
                        for desc in sorted(active_ctrl_desc_to_group.keys()):
                            if desc in next_elem_descriptor:
                                ctrl_groups_to_add.append(
                                    active_ctrl_desc_to_group[desc]
                                )
                            else:
                                missed_group = active_ctrl_desc_to_group[desc]
                                missed_control_groups.add(
                                    f"{current_cell}.{missed_group}"
                                )

                        # add control groups to the stack in order.
                        for ctrl_group in ctrl_groups_to_add:
                            ctrl_group_stack_elem = self._create_ctrl_stack_elem(
                                ctrl_group, cell_component, control_metadata
                            )
                            events_stack_with_ctrl.append(ctrl_group_stack_elem)

                    else:
                        # we are at a cell, and it is the topmost element in the stack.
                        # find parent-child relationships between the control groups
                        # FIXME: this is becoming a code clone with _create_stacks_for_missed_control_groups()
                        active_control_group_to_parents: defaultdict[str, list[str]]
                        leaf_control_groups: set[str]
                        (active_control_group_to_parents, leaf_control_groups) = (
                            self._compute_ctrl_group_to_parents(
                                active_control_group_to_desc
                            )
                        )

                        for leaf_group in leaf_control_groups:
                            leaf_group_split = leaf_group.split(".")
                            leaf_group_name: str = leaf_group_split[-1]
                            leaf_group_cell: str = ".".join(leaf_group_split[:-1])
                            if leaf_group_cell != current_cell:
                                # filter out any control groups that are not in the current cell
                                # (leaf control groups in ancestor cells)
                                continue
                            leaf_stack: list[StackElement] = (
                                events_stack_with_ctrl.copy()
                            )
                            for leaf_parent in active_control_group_to_parents[
                                leaf_group
                            ]:
                                parent_stack_elem: StackElement = (
                                    self._create_ctrl_stack_elem(
                                        leaf_parent.split(".")[-1],
                                        cell_component,
                                        control_metadata,
                                    )
                                )
                                leaf_stack.append(parent_stack_elem)
                            # add leaf
                            leaf_element: StackElement = self._create_ctrl_stack_elem(
                                leaf_group_name, cell_component, control_metadata
                            )
                            leaf_stack.append(leaf_element)
                            # add new stack to current cycle's CycleTrace
                            events_stacks_to_add.append(leaf_stack)
        if len(events_stacks_to_add) == 0:
            events_stacks_to_add.append(events_stack_with_ctrl)

        return events_stacks_to_add, missed_control_groups

    def _create_ctrl_stack_elem(
        self, ctrl_group: str, cell_component: str, control_metadata: ControlMetadata
    ):
        """
        Helper method to create a StackElement for the control group `ctrl_group` (not fully qualified)
        from cell `cell_component`.
        """
        ctrl_group_stack_elem = StackElement(ctrl_group, StackElementType.CONTROL_GROUP)
        # grab the source location string if possible
        if (
            control_metadata.component_to_ctrl_group_to_pos_str is not None
            and ctrl_group
            in control_metadata.component_to_ctrl_group_to_pos_str[cell_component]
        ):
            ctrl_group_stack_elem.ctrl_loc_str = (
                control_metadata.component_to_ctrl_group_to_pos_str[cell_component][
                    ctrl_group
                ]
            )

        return ctrl_group_stack_elem

    def add_sourceloc_info(self, adl_map: AdlMap):
        """
        Wrapper function to add SourceLoc info to elements in self.trace
        """
        trace: PTrace = self.trace_with_control_groups
        assert len(trace) > 0  # can't add sourceloc info on an empty trace

        for i in trace:
            trace[i].add_sourceloc_info(adl_map)

        return trace
