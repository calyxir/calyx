use std::ops::Index;

use crate::flatten::structures::{
    index_trait::{IndexRange, IndexRef},
    indexed_map::IndexedMap,
};

use super::{control::structures::ControlIdx, prelude::*};

#[derive(Debug, Clone)]
pub struct BaseIndexes {
    port_base: LocalPortRef,
    cell_base: LocalCellRef,
    rport_base: LocalRPortRef,
    rcell_base: LocalRCellRef,
}

impl BaseIndexes {
    pub fn new(
        port_base: LocalPortRef,
        cell_base: LocalCellRef,
        rport_base: LocalRPortRef,
        rcell_base: LocalRCellRef,
    ) -> Self {
        Self {
            port_base,
            cell_base,
            rport_base,
            rcell_base,
        }
    }

    pub fn port_base(&self) -> LocalPortRef {
        self.port_base
    }

    pub fn cell_base(&self) -> LocalCellRef {
        self.cell_base
    }

    pub fn rport_base(&self) -> LocalRPortRef {
        self.rport_base
    }

    pub fn rcell_base(&self) -> LocalRCellRef {
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
    pub inputs: IndexRange<LocalPortRef>,
    pub outputs: IndexRange<LocalPortRef>,

    /// all ports nested underneath this component, including the sub-components
    pub total_port_range: IndexRange<LocalPortRef>,
}

impl Default for AuxillaryComponentInfo {
    fn default() -> Self {
        Self {
            name: Identifier::get_default_id(),
            inputs: IndexRange::empty_interval(),
            outputs: IndexRange::empty_interval(),
            total_port_range: IndexRange::empty_interval(),
        }
    }
}

impl AuxillaryComponentInfo {
    /// Creates a new [`AuxillaryComponentInfo`] with the given name. And
    /// default values elsewhere.
    pub fn new_with_name(id: Identifier) -> Self {
        Self {
            name: id,
            inputs: IndexRange::empty_interval(),
            outputs: IndexRange::empty_interval(),
            total_port_range: IndexRange::empty_interval(),
        }
    }
}

#[derive(Debug)]
pub struct PortNames {
    pub port_names: IndexedMap<LocalPortRef, Identifier>,
    pub ref_port_names: IndexedMap<LocalRPortRef, Identifier>,
}

impl PortNames {
    /// Creates a new [`CompNames`] struct with the default value for the
    /// auxillary maps being the empty string.
    pub fn new() -> Self {
        Self {
            port_names: IndexedMap::new(),
            ref_port_names: IndexedMap::new(),
        }
    }
}

impl Default for PortNames {
    fn default() -> Self {
        Self::new()
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

impl Index<LocalPortRef> for CompactLocalNameMap {
    type Output = Identifier;

    fn index(&self, index: LocalPortRef) -> &Self::Output {
        debug_assert!(self.port_base != usize::MAX);
        &self.names[self.port_base + index.index()]
    }
}

impl Index<LocalRPortRef> for CompactLocalNameMap {
    type Output = Identifier;

    fn index(&self, index: LocalRPortRef) -> &Self::Output {
        debug_assert!(self.rport_base != usize::MAX);
        &self.names[self.rport_base + index.index()]
    }
}

impl Index<LocalRCellRef> for CompactLocalNameMap {
    type Output = Identifier;

    fn index(&self, index: LocalRCellRef) -> &Self::Output {
        debug_assert!(self.rcell_base != usize::MAX);
        &self.names[self.rcell_base + index.index()]
    }
}

impl Index<LocalCellRef> for CompactLocalNameMap {
    type Output = Identifier;

    fn index(&self, index: LocalCellRef) -> &Self::Output {
        debug_assert!(self.cell_base != usize::MAX);
        &self.names[self.cell_base + index.index()]
    }
}
