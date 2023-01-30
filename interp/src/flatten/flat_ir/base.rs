// making these all u32 for now, can give the macro an optional type as the
// second arg to contract or expand as needed

use crate::flatten::structures::{
    index_trait::impl_index, indexed_map::IndexedMap,
};

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

impl_index!(pub AssignmentIdx);

impl_index!(pub GroupIdx);

impl_index!(pub CombGroupIdx);

impl_index!(pub GuardIdx);
