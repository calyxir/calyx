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

    def add_signal_prefix(self, signal_prefix):
        """
        Add OS-specific Verilator prefix to all cell names
        """
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
    # component --> { fsms in the component }
    component_to_fsms: dict[str, set[str]] = field(default_factory=dict)
    # fully qualified par name --> [fully-qualified par children name]
    par_to_children: dict[str, list[str]] = field(default_factory=dict)
    # # fully qualified par name --> [fully-qualified par parent name]
    # par_to_par_parent: dict[str, list[str]] = field(default_factory=dict)
    cell_to_child_to_par_parent: dict[str, dict[str, ParChildInfo]] = field(default_factory=dict)
    # fully qualified names of done registers for pars
    par_done_regs: set[str] = field(default_factory=set)
    # partial_fsm_events: 


    def add_signal_prefix_to_fsm(self, signal_prefix):
        self.fsms = {f"{signal_prefix}.{fsm}" for fsm in self.fsms}
        self.par_done_regs = {f"{signal_prefix}.{pd}" for pd in self.par_done_regs}
        self.par_groups = {f"{signal_prefix}.{par_group}" for par_group in self.par_groups}
