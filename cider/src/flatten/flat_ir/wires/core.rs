use cider_idx::{iter::IndexRange, maps::IndexedMap};

use crate::flatten::flat_ir::prelude::*;

/// A map storing all the assignments defined by the program either explicitly
/// or implicitly
pub type AssignmentMap = IndexedMap<AssignmentIdx, Assignment>;
/// A map storing all the groups defined by the program
pub type GroupMap = IndexedMap<GroupIdx, Group>;
/// A map storing all the combinational groups defined by the program
pub type CombGroupMap = IndexedMap<CombGroupIdx, CombGroup>;

/// An assignment in the program. Analogue of [calyx_ir::Assignment]
#[derive(Debug)]
pub struct Assignment {
    /// The destination of the assignment
    pub dst: PortRef,
    /// The source of the assignment
    pub src: PortRef,
    /// The assignment's guard
    pub guard: GuardIdx,
}

/// A group in the program. Analogue of [calyx_ir::Group]
#[derive(Debug)]
pub struct Group {
    /// The name of the group
    name: Identifier,
    /// the assignments in this group
    pub assignments: IndexRange<AssignmentIdx>,
    /// the go signal for this group
    pub go: LocalPortOffset,
    /// the done signal for this group
    pub done: LocalPortOffset,
}

impl Group {
    /// Create a new group
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

    /// Get the name of the group
    pub fn name(&self) -> Identifier {
        self.name
    }
}

/// A combinational group in the program. Analogue of [calyx_ir::CombGroup]
#[derive(Debug)]
pub struct CombGroup {
    name: Identifier,
    /// the assignments in this group
    pub assignments: IndexRange<AssignmentIdx>,
}

impl CombGroup {
    /// Create a new combinational group
    pub fn new(
        name: Identifier,
        assignments: IndexRange<AssignmentIdx>,
    ) -> Self {
        Self { name, assignments }
    }

    /// Get the name of the group
    pub fn name(&self) -> Identifier {
        self.name
    }
}
