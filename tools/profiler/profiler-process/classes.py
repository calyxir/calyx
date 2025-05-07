from dataclasses import dataclass, field
from enum import Enum

from errors import ProfilerException

@dataclass
class CellMetadata:
    """
    Preprocessed data relating to cells
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
    main_shortname: str = None
    signal_prefix: str = None

    def add_signal_prefix(self, signal_prefix: str):
        """
        Add OS-specific Verilator prefix to all cell names
        """
        self.signal_prefix = signal_prefix
        str_to_add = signal_prefix + "."
        self.main_component = str_to_add + self.main_component
        
        for cell in list(self.cell_to_component.keys()):
            fully_qualified_cell = str_to_add + cell
            self.cell_to_component[fully_qualified_cell] = self.cell_to_component[cell]
            del self.cell_to_component[cell]

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

@dataclass
class ParChildInfo:
    child_name: str
    parent: str
    child_type: ParChildType

@dataclass
class ControlMetadata:
    # names of fully qualified FSMs
    fsms: set[str] = field(default_factory=set)
    # names of fully qualified par groups
    par_groups: set[str] = field(default_factory=set)
    # component --> { fsm in the component. NOT fully qualified }
    component_to_fsms: dict[str, set[str]] = field(default_factory=dict)
    # component --> { par groups in the component }
    component_to_par_groups: dict[str, set[str]] = field(default_factory=dict)
    # # fully qualified par name --> [fully-qualified par children name]
    # par_to_children: dict[str, list[str]] = field(default_factory=dict)
    # component --> { child name --> ParChildInfo (contains parent name and child type) }
    component_to_child_to_par_parent: dict[str, dict[str, set[ParChildInfo]]] = field(default_factory=dict)
    # fully qualified names of done registers for pars
    par_done_regs: set[str] = field(default_factory=set)
    # partial_fsm_events: 

    cell_to_ordered_pars: dict[str, list[str]] = field(default_factory=dict) # cell --> ordered par group names

    # FIXME: see if we want to bring this back
    # # fully qualified par name --> [fully-qualified par parent name]
    # par_to_par_parent: dict[str, list[str]] = field(default_factory=dict)
    # this is necessary because if a nested par occurs simultaneously with a group, we don't want the nested par to be a parent of the group

    def add_par_done_reg(self, par_done_reg):
        self.par_done_regs.add(par_done_reg)

    def register_fully_qualified_par(self, fully_qualified_par):
        self.par_groups.add(fully_qualified_par)

    def add_signal_prefix(self, signal_prefix: str):
        self.fsms = {f"{signal_prefix}.{fsm}" for fsm in self.fsms}
        self.par_done_regs = {f"{signal_prefix}.{pd}" for pd in self.par_done_regs}
        self.par_groups = {f"{signal_prefix}.{par_group}" for par_group in self.par_groups}
    
    def register_fsm(self, fsm_name, component, cell_metadata: CellMetadata):
        """
        Add information about a newly discovered FSM to the fields fsms and component_to_fsms.
        """
        if component not in cell_metadata.component_to_cells:
            # skip FSMs from components listed in primitive files (not in user-defined code)
            return
        if component in self.component_to_fsms:            
            self.component_to_fsms[component].add(fsm_name)
        else:
            self.component_to_fsms[component] = set(fsm_name)

        for cell in cell_metadata.component_to_cells[component]:
            fully_qualified_fsm = ".".join((cell, fsm_name))
            self.fsms.add(fully_qualified_fsm)

    def register_par(self, par_group, component):
        if component not in self.component_to_par_groups:
            self.component_to_par_groups[component] = {par_group}
        else:
            self.component_to_par_groups[component].add(par_group)

    def register_par_child(self, component, parent_info):
        """
        Add information about a par child to the field component_to_child_to_par_parent.
        """
        child_name = parent_info.child
        if component in self.component_to_child_to_par_parent:
            if child_name in self.component_to_child_to_par_parent[component]:
                self.component_to_child_to_par_parent[component][child_name].add(parent_info)
            else:
                self.component_to_child_to_par_parent[component][child_name] = {parent_info}
        else:
            self.component_to_child_to_par_parent[component] = {child_name: {parent_info}}

    def order_pars(self, cell_metadata: CellMetadata):
        """
        Give a partial ordering for pars so we know when multiple pars occur simultaneously, what order
        we should add them to the trace.
        (1) order based on cells
        (2) for pars in the same cell, order based on dependencies information
        """
        for cell in sorted(cell_metadata.cell_to_component.keys(), key=(lambda c: c.count("."))):
            self.cell_to_ordered_pars[cell] = []
            component = cell_metadata.cell_to_component[cell]
            pars = self.component_to_par_groups[component]
            # start with pars with no parent
            pars_with_parent = filter((lambda x: self.component_to_child_to_par_parent[component][x].child_type == ParChildType.PAR), self.component_to_child_to_par_parent[component])
            worklist = list(pars.difference(pars_with_parent))
            while len(worklist) > 0:
                par = worklist.pop(0)
                if par not in self.cell_to_ordered_pars[cell]:
                    self.cell_to_ordered_pars[cell].append(par)  # f"{signal_prefix}.{par}"
                # get all the children of this par
                worklist += self.par_to_children[par]

class CycleType(Enum):
    GROUP_OR_PRIMITIVE = 1
    FSM_UPDATE = 2
    PD_UPDATE = 3
    MULT_CONTROL = 4
    OTHER = 5

class StackElementType(Enum):
    GROUP = 1
    PRIMITIVE = 2
    CELL = 3
    CONTROL_GROUP = 4 # TDCC-generated groups that manage control

@dataclass
class StackElement:
    name: str
    element_type: StackElementType
    is_main: bool = field(default=False)
    component_name: str = field(default=None) # should only contain a value if element_type is CELL
    replacement_cell_name: str = field(default=None) # should only contain a value if element_type is CELL

    def __repr__(self):
        match self.element_type:
            case StackElementType.GROUP:
                return self.name
            case StackElementType.PRIMITIVE:
                return f"{self.name} (primitive)"
            case StackElementType.CELL:
                if self.replacement_cell_name is not None:
                    return f"{self.name} ({self.replacement_cell_name}) [{self.component_name}]"
                else:
                    return f"{self.name} [{self.component_name}]"
            case StackElementType.CONTROL_GROUP:
                return f"{self.name} (ctrl)"

class CycleTrace:
    """
    List of stacks that are active in a particular cycle
    """
    stacks: list[list[StackElement]]
    is_useful_cycle: bool
    cycle_type: CycleType

    def __init__(self, stacks_this_cycle: list[list[StackElement]]):
        self.stacks = stacks_this_cycle

        # If a group or primitive is at the top of at least one stack, then the cycle is "useful"
        self.is_useful_cycle = False
        for stack in self.stacks:
            top: StackElement = stack[-1]
            match top.element_type:
                case CycleType.GROUP_OR_PRIMITIVE:
                    self.is_useful_cycle = True

    def __repr__(self):
        out = ""
        for stack in self.stacks:
            out += f"\t{stack}\n"
        return out

@dataclass
class Summary:
    """
    Summary for Cells/Control groups on the number of times they were active vs their active cycles
    FIXME: Add min/max/avg and collect these for normal groups as well?
    """
    num_times_active: int = 0
    active_cycles: set = field(default_factory=set)

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
    update_type: ControlRegUpdateType



@dataclass
class TraceData:
    trace: dict[int, CycleTrace] = field(default_factory=dict)
    trace_classified: dict[int, CycleType] = field(default_factory=dict)
    cell_to_active_cycles: dict[str, Summary] = field(default_factory=dict)

    # fields relating to control groups/registers
    trace_with_control_groups: dict[int, CycleTrace] = field(default_factory=dict)
    control_group_to_active_cycles: dict[str, Summary] = field(default_factory=dict)
    control_reg_updates: dict[str, list[ControlRegUpdates]] = field(default_factory=dict) # cell --> ControlRegUpdate. This is for constructing timeline later.

    def print_trace(self, threshold=-1):
        """
        Threshold is an optional argument that determines how many cycles you are going to print out.
        """
        if threshold == 0:
            return
        for i in self.trace:
            if threshold > 0 and threshold < i:
                return
            print(i)
            print(self.trace[i])

    def incr_num_times_active(self, name: str, d: dict[str, Summary]):
        if name not in d:
            d[name] = Summary()
        d[name].num_times_active += 1

    def cell_start_invoke(self, cell: str):
        self.incr_num_times_active(cell, self.cell_to_active_cycles)
    
    def register_cell_cycle(self, cell, cycle):
        self.cell_to_active_cycles[cell].active_cycles.add(cycle)

    def control_group_interval(self, group: str, interval: range):
        self.incr_num_times_active(group, self.control_group_to_active_cycles)
        self.control_group_to_active_cycles[group].active_cycles.add(interval)

    def register_control_reg_update(self, cell: str, clock_cycle: int, update_str: str):
        if cell not in self.control_reg_updates:
            self.control_reg_updates[cell] = []
        self.control_reg_updates[cell].append(ControlRegUpdates(cell, clock_cycle, update_str))

    def create_trace_with_control_groups(self, control_groups_trace: dict[int, set[str]], cell_metadata: CellMetadata, control_metadata: ControlMetadata):
        control_metadata.order_pars(cell_metadata)
        for i in self.trace:
            if i in control_groups_trace:
                for events_stack in self.trace[i].stacks:
                    new_events_stack: list[StackElement] = []
                    for stack_element in events_stack:
                        new_events_stack.append(stack_element)
                        match stack_element.element_type:
                            case StackElementType.CELL:
                                if stack_element.is_main:
                                    current_cell = stack_element.name
                                else:
                                    current_cell += f".{stack_element.name}"
                            case StackElementType.GROUP:
                                # standard groups to handle edge case of nested pars concurrent with groups; pop any pars that aren't this group's parent
                                current_component = cell_metadata.cell_to_component[current_cell]
                                if (current_component in control_metadata.component_to_child_to_par_parent and stack_element.name in control_metadata.component_to_child_to_par_parent[current_component]):
                                    child_parent_infos = control_metadata.component_to_child_to_par_parent[current_component][stack_element.name]
                                    parent_names = set()
                                    for child_parent_info in child_parent_infos:
                                        if child_parent_info.child_type == ParChildType.PAR:
                                            raise ProfilerException("A normal group should not be stored as a par group under control_metadata.component_to_child_to_par_parent")
                                        parent_names.add(child_parent_info.parent)
                                    parent_found = False
                                    while (len(new_events_stack) > 2 and new_events_stack[-2].element_type == StackElementType.CONTROL_GROUP):
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
                        if current_cell in control_metadata.cell_to_ordered_pars:
                            active_from_cell = control_groups_trace[i].intersection(control_metadata.cell_to_ordered_pars[current_cell])
                            for par_group_active in sorted(active_from_cell, key=(
                                    lambda p: control_metadata.cells_to_ordered_pars[current_cell].index(p)
                            )):
                                par_group_name = par_group_active.split(".")[-1]
                                new_events_stack.append(StackElement(par_group_name, StackElementType.CONTROL_GROUP))
                    self.trace_with_control_groups[i].append(new_events_stack)
            else:
                self.trace_with_control_groups[i] = self.trace[i].copy()

