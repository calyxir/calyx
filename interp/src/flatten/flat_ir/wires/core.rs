use crate::flatten::{
    flat_ir::prelude::*,
    structures::{index_trait::IndexRange, indexed_map::IndexedMap},
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
    pub go: LocalPortOffset,
    /// the done signal for this group
    pub done: LocalPortOffset,
}

impl Group {
    pub fn new(
        name: Identifier,
        assignments: IndexRange<AssignmentIdx>,
        go: LocalPortOffset,
        done: LocalPortOffset,
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
    pub fn new(
        name: Identifier,
        assignments: IndexRange<AssignmentIdx>,
    ) -> Self {
        Self { name, assignments }
    }

    pub fn name(&self) -> Identifier {
        self.name
    }
}
