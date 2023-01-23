use smallvec::SmallVec;

use crate::flatten::{
    flat_ir::prelude::*,
    structures::{
        environment::{LocalCellRef, PortRef},
        index_trait::{impl_index, IndexRange},
        indexed_map::IndexedMap,
    },
};

use super::guards::GuardIdx;

impl_index!(pub AssignmentIdx);
pub type AssignmentMap = IndexedMap<Assignment, AssignmentIdx>;

impl_index!(pub GroupIdx);
pub type GroupMap = IndexedMap<Group, GroupIdx>;

impl_index!(pub CombGroupIdx);
pub type CombGroupMap = IndexedMap<CombGroup, CombGroupIdx>;

#[derive(Debug)]
pub struct Assignment {
    pub dst: PortRef,
    pub src: PortRef,
    pub guard: GuardIdx,
}

#[derive(Debug)]
pub struct Group {
    name: Identifier,
    /// the assignments in this group
    pub assignments: IndexRange<AssignmentIdx>,
    /// the go signal for this group
    pub go: LocalCellRef,
    /// the done signal for this group
    pub done: LocalCellRef,
}

#[derive(Debug)]
pub struct CombGroup {
    name: Identifier,
    /// the assignments in this group
    pub assignments: IndexRange<AssignmentIdx>,
}
