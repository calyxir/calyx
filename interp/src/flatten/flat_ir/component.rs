use super::super::structures::context::Context;
use crate::flatten::structures::{
    index_trait::{IndexRange, SignatureRange},
    indexed_map::IndexedMap,
    sparse_map::SparseMap,
};

use super::{control::structures::ControlIdx, prelude::*};

#[derive(Debug, Clone)]
pub struct DefinitionRanges {
    cells: IndexRange<CellDefinitionIdx>,
    ports: IndexRange<PortDefinitionIdx>,
    ref_cells: IndexRange<RefCellDefinitionIdx>,
    ref_ports: IndexRange<RefPortDefinitionIdx>,
    groups: IndexRange<GroupIdx>,
    comb_groups: IndexRange<CombGroupIdx>,
}

impl DefinitionRanges {
    pub fn cells(&self) -> &IndexRange<CellDefinitionIdx> {
        &self.cells
    }

    pub fn ports(&self) -> &IndexRange<PortDefinitionIdx> {
        &self.ports
    }

    pub fn ref_cells(&self) -> &IndexRange<RefCellDefinitionIdx> {
        &self.ref_cells
    }

    pub fn ref_ports(&self) -> &IndexRange<RefPortDefinitionIdx> {
        &self.ref_ports
    }

    pub fn groups(&self) -> &IndexRange<GroupIdx> {
        &self.groups
    }

    pub fn comb_groups(&self) -> &IndexRange<CombGroupIdx> {
        &self.comb_groups
    }
}

impl Default for DefinitionRanges {
    fn default() -> Self {
        Self {
            ports: IndexRange::empty_interval(),
            ref_ports: IndexRange::empty_interval(),
            cells: IndexRange::empty_interval(),
            ref_cells: IndexRange::empty_interval(),
            groups: IndexRange::empty_interval(),
            comb_groups: IndexRange::empty_interval(),
        }
    }
}

/// A structure which contains the basic information about a component
/// definition needed during simulation.
#[derive(Debug)]
pub struct ComponentCore {
    /// The control program for this component.
    pub control: Option<ControlIdx>,
    /// The set of assignments that are always active.
    pub continuous_assignments: IndexRange<AssignmentIdx>,
    /// True iff component is combinational
    pub is_comb: bool,
    /// The go port for this component
    pub go: LocalPortOffset,
    /// The done port for this component
    pub done: LocalPortOffset,
}

pub enum AssignmentDefinitionLocation {
    /// The assignment is contained in a comb group
    CombGroup(CombGroupIdx),
    /// The assignment is contained in a group
    Group(GroupIdx),
    /// The assignment is one of the continuous assignments for the component
    ContinuousAssignment,
    /// The assignment is implied by an invoke
    Invoke(ControlIdx),
}

impl ComponentCore {
    /// Returns true if the given assignment is contained in this component.
    ///
    /// Note: This is not a very efficient implementation since it's doing a
    /// DFS search over the control tree.
    pub fn contains_assignment(
        &self,
        ctx: &Context,
        assign: AssignmentIdx,
    ) -> Option<AssignmentDefinitionLocation> {
        if self.continuous_assignments.contains(assign) {
            return Some(AssignmentDefinitionLocation::ContinuousAssignment);
        } else if let Some(root) = self.control {
            let mut search_stack = vec![root];
            while let Some(node) = search_stack.pop() {
                match &ctx.primary[node] {
                    ControlNode::Empty(_) => {}
                    ControlNode::Enable(e) => {
                        if ctx.primary[e.group()].assignments.contains(assign) {
                            return Some(AssignmentDefinitionLocation::Group(
                                e.group(),
                            ));
                        }
                    }
                    ControlNode::Seq(s) => {
                        for stmt in s.stms() {
                            search_stack.push(*stmt);
                        }
                    }
                    ControlNode::Par(p) => {
                        for stmt in p.stms() {
                            search_stack.push(*stmt);
                        }
                    }
                    ControlNode::If(i) => {
                        if let Some(comb) = i.cond_group() {
                            if ctx.primary[comb].assignments.contains(assign) {
                                return Some(
                                    AssignmentDefinitionLocation::CombGroup(
                                        comb,
                                    ),
                                );
                            }
                        }

                        search_stack.push(i.tbranch());
                        search_stack.push(i.fbranch());
                    }
                    ControlNode::While(wh) => {
                        if let Some(comb) = wh.cond_group() {
                            if ctx.primary[comb].assignments.contains(assign) {
                                return Some(
                                    AssignmentDefinitionLocation::CombGroup(
                                        comb,
                                    ),
                                );
                            }
                        }
                        search_stack.push(wh.body());
                    }
                    ControlNode::Repeat(r) => {
                        search_stack.push(r.body);
                    }
                    ControlNode::Invoke(i) => {
                        if let Some(comb) = i.comb_group {
                            if ctx.primary[comb].assignments.contains(assign) {
                                return Some(
                                    AssignmentDefinitionLocation::CombGroup(
                                        comb,
                                    ),
                                );
                            }
                        }

                        if i.assignments.contains(assign) {
                            return Some(AssignmentDefinitionLocation::Invoke(
                                node,
                            ));
                        }
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
/// Other information about a component definition. This is not on the hot path
/// and is instead needed primarily during setup and error reporting.
pub struct AuxillaryComponentInfo {
    /// Name of the component.
    pub name: Identifier,
    /// The input/output signature of this component. This could probably be
    /// rolled into a single range, or specialized construct but this is
    /// probably fine for now.
    pub signature_in: SignatureRange,
    pub signature_out: SignatureRange,

    /// the definitions created by this component
    pub definitions: DefinitionRanges,

    pub port_offset_map: SparseMap<LocalPortOffset, PortDefinitionIdx>,
    pub ref_port_offset_map:
        SparseMap<LocalRefPortOffset, RefPortDefinitionIdx>,
    pub cell_offset_map: SparseMap<LocalCellOffset, CellDefinitionIdx>,
    pub ref_cell_offset_map:
        SparseMap<LocalRefCellOffset, RefCellDefinitionIdx>,
}

impl Default for AuxillaryComponentInfo {
    fn default() -> Self {
        Self::new_with_name(Identifier::get_default_id())
    }
}

impl AuxillaryComponentInfo {
    /// Creates a new [`AuxillaryComponentInfo`] with the given name. And
    /// default values elsewhere.
    pub fn new_with_name(id: Identifier) -> Self {
        Self {
            name: id,
            signature_in: SignatureRange::new(),
            signature_out: SignatureRange::new(),
            port_offset_map: Default::default(),
            ref_port_offset_map: Default::default(),
            cell_offset_map: Default::default(),
            ref_cell_offset_map: Default::default(),
            definitions: Default::default(),
        }
    }

    pub fn set_port_range(
        &mut self,
        start: PortDefinitionIdx,
        end: PortDefinitionIdx,
    ) {
        self.definitions.ports = IndexRange::new(start, end)
    }

    pub fn set_ref_port_range(
        &mut self,
        start: RefPortDefinitionIdx,
        end: RefPortDefinitionIdx,
    ) {
        self.definitions.ref_ports = IndexRange::new(start, end)
    }

    pub fn set_cell_range(
        &mut self,
        start: CellDefinitionIdx,
        end: CellDefinitionIdx,
    ) {
        self.definitions.cells = IndexRange::new(start, end)
    }

    pub fn set_ref_cell_range(
        &mut self,
        start: RefCellDefinitionIdx,
        end: RefCellDefinitionIdx,
    ) {
        self.definitions.ref_cells = IndexRange::new(start, end)
    }

    pub fn set_group_range(&mut self, start: GroupIdx, end: GroupIdx) {
        self.definitions.groups = IndexRange::new(start, end)
    }

    pub fn set_comb_group_range(
        &mut self,
        start: CombGroupIdx,
        end: CombGroupIdx,
    ) {
        self.definitions.comb_groups = IndexRange::new(start, end)
    }

    pub fn inputs(&self) -> impl Iterator<Item = LocalPortOffset> + '_ {
        self.signature_in.iter()
    }

    pub fn outputs(&self) -> impl Iterator<Item = LocalPortOffset> + '_ {
        self.signature_out.iter()
    }

    pub fn signature(&self) -> IndexRange<LocalPortOffset> {
        // can't quite use min here since None is less than any other value and
        // I want the least non-None value
        let beginning =
            match (self.signature_in.first(), self.signature_out.first()) {
                (Some(b), Some(e)) => Some(std::cmp::min(b, e)),
                (Some(b), None) => Some(b),
                (None, Some(e)) => Some(e),
                _ => None,
            };

        let end =
            std::cmp::max(self.signature_in.last(), self.signature_out.last());

        match (beginning, end) {
            (Some(b), Some(e)) => IndexRange::new(b, e),
            (None, None) => IndexRange::empty_interval(),
            _ => unreachable!(),
        }
    }

    fn offset_sizes(&self, cell_ty: ContainmentType) -> IdxSkipSizes {
        let (port, ref_port) = match cell_ty {
            ContainmentType::Local => (
                self.port_offset_map.count() - self.signature().size(),
                self.ref_port_offset_map.count(),
            ),
            ContainmentType::Ref => (
                self.port_offset_map.count(),
                self.ref_port_offset_map.count() - self.signature().size(),
            ),
        };

        IdxSkipSizes {
            port,
            ref_port,
            cell: self.cell_offset_map.count(),
            ref_cell: self.ref_cell_offset_map.count(),
        }
    }

    /// The skip sizes for ref-cell instances of this component
    pub fn skip_sizes_for_ref(&self) -> IdxSkipSizes {
        self.offset_sizes(ContainmentType::Ref)
    }

    /// The skip sizes for non-ref cell instances of this component
    pub fn skip_sizes_for_local(&self) -> IdxSkipSizes {
        self.offset_sizes(ContainmentType::Local)
    }

    pub fn skip_offsets(
        &mut self,
        IdxSkipSizes {
            port,
            ref_port,
            cell,
            ref_cell,
        }: IdxSkipSizes,
    ) {
        self.port_offset_map.skip(port);
        self.ref_port_offset_map.skip(ref_port);
        self.cell_offset_map.skip(cell);
        self.ref_cell_offset_map.skip(ref_cell);
    }

    pub fn get_cell_info_idx(&self, cell: CellRef) -> CellDefinitionRef {
        match cell {
            CellRef::Local(l) => self.cell_offset_map[l].into(),
            CellRef::Ref(r) => self.ref_cell_offset_map[r].into(),
        }
    }
}

pub struct IdxSkipSizes {
    port: usize,
    ref_port: usize,
    cell: usize,
    ref_cell: usize,
}

impl IdxSkipSizes {
    pub fn port(&self) -> usize {
        self.port
    }

    pub fn ref_port(&self) -> usize {
        self.ref_port
    }

    pub fn cell(&self) -> usize {
        self.cell
    }

    pub fn ref_cell(&self) -> usize {
        self.ref_cell
    }
}

pub type ComponentMap = IndexedMap<ComponentIdx, ComponentCore>;
