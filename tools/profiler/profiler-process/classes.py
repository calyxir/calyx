from dataclasses import dataclass, field
from enum import Enum

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
    fsms: set = field(default_factory=set)
    # component --> { fsm in the component. NOT fully qualified }
    component_to_fsms: dict[str, set[str]] = field(default_factory=dict)
    # component --> { par groups in the component }
    # fully qualified par name --> [fully-qualified par children name]
    par_to_children: dict[str, list[str]] = field(default_factory=dict)
    # component --> { child name --> ParChildInfo (contains parent name and child type) }
    component_to_child_to_par_parent: dict[str, dict[str, ParChildInfo]] = field(default_factory=dict)
    # fully qualified names of done registers for pars
    par_done_regs: set[str] = field(default_factory=set)
    # partial_fsm_events: 

    # FIXME: see if we want to bring this back
    # # fully qualified par name --> [fully-qualified par parent name]
    # par_to_par_parent: dict[str, list[str]] = field(default_factory=dict)
    # this is necessary because if a nested par occurs simultaneously with a group, we don't want the nested par to be a parent of the group

    def add_signal_prefix(self, signal_prefix: str):
        self.fsms = {f"{signal_prefix}.{fsm}" for fsm in self.fsms}
        self.par_done_regs = {f"{signal_prefix}.{pd}" for pd in self.par_done_regs}
        self.par_groups = {f"{signal_prefix}.{par_group}" for par_group in self.par_groups}

    def pars(self):
        return set(self.par_to_children.keys())
    
    def register_fsm(self, fsm_name, component, cell_metadata):
        """
        Add information about a newly discovered FSM to the fields fsms and component_to_fsms.
        """
        if component not in cell_metadata.components_to_cells:
            # skip FSMs from components listed in primitive files (not in user-defined code)
            return
        if component in self.component_to_fsms:            
            self.component_to_fsms[component].add(fsm_name)
        else:
            self.component_to_fsms[component] = set(fsm_name)

        for cell in cell_metadata.components_to_cells[component]:
            fully_qualified_fsm = ".".join((cell, fsm_name))
            self.fsms.add(fully_qualified_fsm)

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
    component_name: str = field(default=None) # should only contain a value if element_type is CELL

@dataclass
class CellSummary:
    num_times_active: int
    active_cycles: set

@dataclass
class TraceData:
    trace: dict[int, list[StackElement]]
    trace_classified: dict[int, CycleType]
    cell_to_active_cycles: dict[str, CellSummary]