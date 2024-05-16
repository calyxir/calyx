use crate::flatten::structures::{
    index_trait::IndexRange, indexed_map::IndexedMap, sparse_map::SparseMap,
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
}

#[derive(Debug, Clone)]
/// Other information about a component definition. This is not on the hot path
/// and is instead needed primarily during setup and error reporting.
pub struct AuxillaryComponentInfo {
    /// Name of the component.
    pub name: Identifier,
    /// The input/output signature of this component.
    pub signature: IndexRange<LocalPortOffset>,
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
            signature: IndexRange::empty_interval(),
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

    fn offset_sizes(&self, cell_ty: ContainmentType) -> IdxSkipSizes {
        let (port, ref_port) = match cell_ty {
            ContainmentType::Local => (
                self.port_offset_map.count() - self.signature.size(),
                self.ref_port_offset_map.count(),
            ),
            ContainmentType::Ref => (
                self.port_offset_map.count(),
                self.ref_port_offset_map.count() - self.signature.size(),
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
