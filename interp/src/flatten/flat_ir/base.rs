// making these all u32 for now, can give the macro an optional type as the
// second arg to contract or expand as needed

use std::ops::Index;

use crate::flatten::structures::{
    index_trait::{impl_index, impl_index_nonzero, IndexRange},
    indexed_map::{AuxillaryMap, IndexedMap},
};

use super::prelude::Identifier;

impl_index!(pub ComponentRef);

// Reference for a port assuming a zero base, ie local to the component
impl_index!(pub LocalPortRef);
// Global port reference, used for value mapping
impl_index!(pub GlobalPortRef);
// Global mapping for cell state
impl_index!(pub GlobalCellRef);
// cell reference local to a given component definition
impl_index!(pub LocalCellRef);
// A local reference
impl_index!(pub CellPortID);
// ref cell index
impl_index!(pub GlobalRCellRef);
impl_index!(pub LocalRCellRef);
impl_index!(pub LocalRPortRef);

pub struct RelativePortIdx(u32);
pub struct RelativeRefPortIdx(u32);
pub struct RelativeCellIdx(u32);
pub struct RelativeRefCellIdx(u32);

#[derive(Debug, Copy, Clone)]
pub enum PortRef {
    Local(LocalPortRef),
    Ref(LocalRPortRef),
}

impl PortRef {
    #[must_use]
    pub fn as_local(&self) -> Option<&LocalPortRef> {
        if let Self::Local(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_ref(&self) -> Option<&LocalRPortRef> {
        if let Self::Ref(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn unwrap_local(&self) -> &LocalPortRef {
        self.as_local().unwrap()
    }

    pub fn unwrap_ref(&self) -> &LocalRPortRef {
        self.as_ref().unwrap()
    }
}

impl From<LocalRPortRef> for PortRef {
    fn from(v: LocalRPortRef) -> Self {
        Self::Ref(v)
    }
}

impl From<LocalPortRef> for PortRef {
    fn from(v: LocalPortRef) -> Self {
        Self::Local(v)
    }
}

impl_index!(pub AssignmentIdx);

impl_index!(pub GroupIdx);

// This is non-zero to make the option-types of this index used in the IR If and
// While nodes the same size as the index itself.
impl_index_nonzero!(pub CombGroupIdx);

impl_index!(pub GuardIdx);

#[derive(Debug, Clone)]
pub struct RefCellInfo {
    name: Identifier,
    ports: IndexRange<LocalRPortRef>,
}

impl RefCellInfo {
    pub fn new(name: Identifier, ports: IndexRange<LocalRPortRef>) -> Self {
        Self { name, ports }
    }

    pub fn name(&self) -> Identifier {
        self.name
    }

    pub fn ports(&self) -> &IndexRange<LocalRPortRef> {
        &self.ports
    }
}

#[derive(Debug, Clone)]
pub struct LocalCellInfo {
    name: Identifier,
    ports: IndexRange<LocalPortRef>,
}

impl LocalCellInfo {
    pub fn new(name: Identifier, ports: IndexRange<LocalPortRef>) -> Self {
        Self { name, ports }
    }

    pub fn name(&self) -> Identifier {
        self.name
    }

    pub fn ports(&self) -> &IndexRange<LocalPortRef> {
        &self.ports
    }
}
