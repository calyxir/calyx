// making these all u32 for now, can give the macro an optional type as the
// second arg to contract or expand as needed

use std::ops::Index;

use crate::flatten::structures::{
    index_trait::{impl_index, impl_index_nonzero, IndexRange},
    indexed_map::AuxillaryMap,
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

    pub fn unwrap_local(&self) -> &LocalPortRef {
        self.as_local().unwrap()
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
    pub name: Identifier,
    pub ports: IndexRange<LocalRPortRef>,
}

#[derive(Debug, Clone)]
pub struct LocalCellInfo {
    pub name: Identifier,
    pub ports: IndexRange<LocalPortRef>,
}

#[derive(Debug)]
pub struct CellInfoMap {
    pub local_c: AuxillaryMap<LocalCellRef, LocalCellInfo>,
    pub ref_c: AuxillaryMap<LocalRCellRef, RefCellInfo>,
}

impl CellInfoMap {
    pub fn new() -> Self {
        Self {
            // TODO (griffin): Come up with a better default for IndexRange
            local_c: AuxillaryMap::new_with_default(LocalCellInfo {
                name: Identifier::get_default_id(),
                ports: IndexRange::new(0_u32.into(), 0_u32.into()),
            }),
            ref_c: AuxillaryMap::new_with_default(RefCellInfo {
                name: Identifier::get_default_id(),
                ports: IndexRange::new(0_u32.into(), 0_u32.into()),
            }),
        }
    }
}

impl Index<LocalCellRef> for CellInfoMap {
    type Output = LocalCellInfo;

    fn index(&self, index: LocalCellRef) -> &Self::Output {
        &self.local_c[index]
    }
}

impl Index<LocalRCellRef> for CellInfoMap {
    type Output = RefCellInfo;

    fn index(&self, index: LocalRCellRef) -> &Self::Output {
        &self.ref_c[index]
    }
}
