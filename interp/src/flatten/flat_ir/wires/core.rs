use smallvec::SmallVec;

use crate::flatten::{
    flat_ir::prelude::*,
    structures::{
        index_trait::{impl_index, IndexRange},
        indexed_map::IndexedMap,
    },
};

pub type AssignmentMap = IndexedMap<Assignment, AssignmentIdx>;
pub type GroupMap = IndexedMap<Group, GroupIdx>;
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

impl Group {
    pub fn name(&self) -> Identifier {
        self.name
    }
}

#[derive(Debug)]
pub struct CombGroup {
    name: Identifier,
    /// the assignments in this group
    pub assignments: IndexRange<AssignmentIdx>,
}

impl CombGroup {
    pub fn name(&self) -> Identifier {
        self.name
    }
}
