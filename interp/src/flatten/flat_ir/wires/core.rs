use smallvec::SmallVec;

use crate::flatten::{
    flat_ir::prelude::*,
    structures::{
        index_trait::{impl_index, IndexRange},
        indexed_map::IndexedMap,
    },
};

pub type AssignmentMap = IndexedMap<AssignmentIdx, Assignment>;
pub type GroupMap = IndexedMap<GroupIdx, Group>;
pub type CombGroupMap = IndexedMap<CombGroupIdx, CombGroup>;

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
    pub go: LocalPortRef,
    /// the done signal for this group
    pub done: LocalPortRef,
}

impl Group {
    pub fn new(
        name: Identifier,
        assignments: IndexRange<AssignmentIdx>,
        go: LocalPortRef,
        done: LocalPortRef,
    ) -> Self {
        Self {
            name,
            assignments,
            go,
            done,
        }
    }

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
