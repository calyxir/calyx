use std::ops::Index;

use crate::flatten::structures::{
    index_trait::{IndexRange, IndexRef},
    indexed_map::IndexedMap,
    sparse_map::SparseMap,
};

use super::{control::structures::ControlIdx, prelude::*};

#[derive(Debug, Clone)]
pub struct BaseIndexes {
    port_base: PortDefinition,
    cell_base: CellDefinition,
    rport_base: RefPortDefinition,
    rcell_base: RefCellDefinition,
}

impl BaseIndexes {
    pub fn new(
        port_base: PortDefinition,
        cell_base: CellDefinition,
        rport_base: RefPortDefinition,
        rcell_base: RefCellDefinition,
    ) -> Self {
        Self {
            port_base,
            cell_base,
            rport_base,
            rcell_base,
        }
    }

    pub fn port_base(&self) -> PortDefinition {
        self.port_base
    }

    pub fn cell_base(&self) -> CellDefinition {
        self.cell_base
    }

    pub fn rport_base(&self) -> RefPortDefinition {
        self.rport_base
    }

    pub fn rcell_base(&self) -> RefCellDefinition {
        self.rcell_base
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
    /// the base indices for all the local references
    pub indices: BaseIndexes,
}

#[derive(Debug, Clone)]
/// Other information about a component definition. This is not on the hot path
/// and is instead needed primarily during setup and error reporting.
pub struct AuxillaryComponentInfo {
    /// Name of the component.
    pub name: Identifier,
    /// The input/output signature of this component.
    pub signature: IndexRange<PortDefinition>,
    /// all ports nested underneath this component, including the sub-components
    pub total_port_range: IndexRange<PortDefinition>,
    // -------------------
    port_offset_map: SparseMap<LocalPortOffset, PortDefinition>,
    ref_port_offset_map: SparseMap<LocalRefPortOffset, RefPortDefinition>,
    cell_offset_map: SparseMap<LocalCellOffset, CellDefinition>,
    ref_cell_offset_map: SparseMap<LocalRefCellOffset, RefCellDefinition>,
}

impl Default for AuxillaryComponentInfo {
    fn default() -> Self {
        Self {
            name: Identifier::get_default_id(),
            signature: IndexRange::empty_interval(),
            total_port_range: IndexRange::empty_interval(),
            port_offset_map: Default::default(),
            ref_port_offset_map: Default::default(),
            cell_offset_map: Default::default(),
            ref_cell_offset_map: Default::default(),
        }
    }
}

impl AuxillaryComponentInfo {
    /// Creates a new [`AuxillaryComponentInfo`] with the given name. And
    /// default values elsewhere.
    pub fn new_with_name(id: Identifier) -> Self {
        Self {
            name: id,
            signature: IndexRange::empty_interval(),
            total_port_range: IndexRange::empty_interval(),
            port_offset_map: Default::default(),
            ref_port_offset_map: Default::default(),
            cell_offset_map: Default::default(),
            ref_cell_offset_map: Default::default(),
        }
    }
}

pub type ComponentMap = IndexedMap<ComponentRef, ComponentCore>;

// NOTHING IMPORTANT DOWN HERE, DO NOT READ
// =======================================

/// IGNORE FOR NOW. THIS IS UNUSED
///
///  A map from various local references to the name of the port/cell
///
/// The basic idea is to have a single vector of the names densely packed and to
/// have the separate types be distinct regions of the vector.
pub struct CompactLocalNameMap {
    port_base: usize,
    cell_base: usize,
    rport_base: usize,
    rcell_base: usize,
    names: Vec<Identifier>,
}

impl CompactLocalNameMap {
    /// Creates a new [`CompactLocalNameMap`] with the given capacity.
    pub fn with_capacity(size: usize) -> Self {
        Self {
            port_base: usize::MAX,
            cell_base: usize::MAX,
            rport_base: usize::MAX,
            rcell_base: usize::MAX,
            names: Vec::with_capacity(size),
        }
    }
    /// Creates a new [`CompactLocalNameMap`].
    pub fn new() -> Self {
        Self::with_capacity(0)
    }
}

impl Default for CompactLocalNameMap {
    fn default() -> Self {
        Self::new()
    }
}

// Lots index trait implementations, not interesting I promise

impl Index<PortRef> for CompactLocalNameMap {
    type Output = Identifier;

    fn index(&self, index: PortRef) -> &Self::Output {
        match index {
            PortRef::Local(idx) => {
                debug_assert!(self.port_base != usize::MAX);
                &self.names[self.port_base + idx.index()]
            }
            PortRef::Ref(idx) => {
                debug_assert!(self.rport_base != usize::MAX);
                &self.names[self.rport_base + idx.index()]
            }
        }
    }
}

impl Index<PortDefinition> for CompactLocalNameMap {
    type Output = Identifier;

    fn index(&self, index: PortDefinition) -> &Self::Output {
        debug_assert!(self.port_base != usize::MAX);
        &self.names[self.port_base + index.index()]
    }
}

impl Index<RefPortDefinition> for CompactLocalNameMap {
    type Output = Identifier;

    fn index(&self, index: RefPortDefinition) -> &Self::Output {
        debug_assert!(self.rport_base != usize::MAX);
        &self.names[self.rport_base + index.index()]
    }
}

impl Index<RefCellDefinition> for CompactLocalNameMap {
    type Output = Identifier;

    fn index(&self, index: RefCellDefinition) -> &Self::Output {
        debug_assert!(self.rcell_base != usize::MAX);
        &self.names[self.rcell_base + index.index()]
    }
}

impl Index<CellDefinition> for CompactLocalNameMap {
    type Output = Identifier;

    fn index(&self, index: CellDefinition) -> &Self::Output {
        debug_assert!(self.cell_base != usize::MAX);
        &self.names[self.cell_base + index.index()]
    }
}
