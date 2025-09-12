from dataclasses import dataclass, field
from collections import defaultdict
from enum import Enum

from .cell_metadata import CellMetadata


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
