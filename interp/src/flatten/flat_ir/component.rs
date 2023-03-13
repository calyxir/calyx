use crate::flatten::structures::{
    index_trait::IndexRange, indexed_map::IndexedMap, sparse_map::SparseMap,
};

use super::{control::structures::ControlIdx, prelude::*};

#[derive(Debug, Clone)]
pub struct DefinitionRanges {
    cells: IndexRange<CellDefinition>,
    ports: IndexRange<PortDefinition>,
    ref_cells: IndexRange<RefCellDefinition>,
    ref_ports: IndexRange<RefPortDefinition>,
}

impl DefinitionRanges {
    pub fn cells(&self) -> &IndexRange<CellDefinition> {
        &self.cells
    }

    pub fn ports(&self) -> &IndexRange<PortDefinition> {
        &self.ports
    }

    pub fn ref_cells(&self) -> &IndexRange<RefCellDefinition> {
        &self.ref_cells
    }

    pub fn ref_ports(&self) -> &IndexRange<RefPortDefinition> {
        &self.ref_ports
    }
}

impl Default for DefinitionRanges {
    fn default() -> Self {
        Self {
            ports: IndexRange::empty_interval(),
            ref_ports: IndexRange::empty_interval(),
            cells: IndexRange::empty_interval(),
            ref_cells: IndexRange::empty_interval(),
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
    // -------------------
    pub port_offset_map: SparseMap<LocalPortOffset, PortDefinition>,
    pub ref_port_offset_map: SparseMap<LocalRefPortOffset, RefPortDefinition>,
    pub cell_offset_map: SparseMap<LocalCellOffset, CellDefinition>,
    pub ref_cell_offset_map: SparseMap<LocalRefCellOffset, RefCellDefinition>,
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
        start: PortDefinition,
        end: PortDefinition,
    ) {
        self.definitions.ports = IndexRange::new(start, end)
    }

    pub fn set_ref_port_range(
        &mut self,
        start: RefPortDefinition,
        end: RefPortDefinition,
    ) {
        self.definitions.ref_ports = IndexRange::new(start, end)
    }

    pub fn set_cell_range(
        &mut self,
        start: CellDefinition,
        end: CellDefinition,
    ) {
        self.definitions.cells = IndexRange::new(start, end)
    }

    pub fn set_ref_cell_range(
        &mut self,
        start: RefCellDefinition,
        end: RefCellDefinition,
    ) {
        self.definitions.ref_cells = IndexRange::new(start, end)
    }

    pub fn offset_sizes(&self) -> IdxSkipSizes {
        IdxSkipSizes {
            port: self.port_offset_map.count() - self.signature.size(),
            ref_port: self.ref_port_offset_map.count(),
            cell: self.cell_offset_map.count(),
            ref_cell: self.ref_cell_offset_map.count(),
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

pub type ComponentMap = IndexedMap<ComponentRef, ComponentCore>;
