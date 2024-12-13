use crate::flatten::{
    flat_ir::prelude::{GlobalCellIdx, LocalPortOffset},
    structures::thread::{ThreadIdx, ThreadMap},
};

use super::env::AssignmentRange;

#[derive(Debug)]
pub struct GroupInterfacePorts {
    pub go: LocalPortOffset,
    pub done: LocalPortOffset,
}

/// An enum describing the source of a set of assignments
#[derive(Debug, Clone, Copy)]
pub enum AssignType {
    /// Assignments come from a comb group
    Combinational,
    /// Assignments are continuous
    Continuous,
    /// Assignments come from a group or invoke
    Control,
}

impl AssignType {
    /// Returns `true` if the assign source is [`Combinational`].
    ///
    /// [`Combinational`]: AssignType::Combinational
    #[must_use]
    pub fn is_combinational(&self) -> bool {
        matches!(self, Self::Combinational)
    }

    /// Returns `true` if the assign source is [`Continuous`].
    ///
    /// [`Continuous`]: AssignType::Continuous
    #[must_use]
    pub fn is_continuous(&self) -> bool {
        matches!(self, Self::Continuous)
    }

    /// Returns `true` if the assign type is [`Control`].
    ///
    /// [`Control`]: AssignType::Control
    #[must_use]
    pub fn is_control(&self) -> bool {
        matches!(self, Self::Control)
    }
}

/// A group of assignments that is scheduled to be evaluated
#[derive(Debug)]
pub struct ScheduledAssignments {
    pub active_cell: GlobalCellIdx,
    pub assignments: AssignmentRange,
    pub interface_ports: Option<GroupInterfacePorts>,
    pub thread: Option<ThreadIdx>,
    pub assign_type: AssignType,
}

impl ScheduledAssignments {
    pub fn new_control(
        active_cell: GlobalCellIdx,
        assignments: AssignmentRange,
        interface_ports: Option<GroupInterfacePorts>,
        thread: Option<ThreadIdx>,
    ) -> Self {
        Self {
            active_cell,
            assignments,
            interface_ports,
            thread,
            assign_type: AssignType::Control,
        }
    }

    pub fn new_combinational(
        active_cell: GlobalCellIdx,
        assignments: AssignmentRange,
    ) -> Self {
        Self {
            active_cell,
            assignments,
            interface_ports: None,
            thread: None,
            assign_type: AssignType::Combinational,
        }
    }

    pub fn new_continuous(
        active_cell: GlobalCellIdx,
        assignments: AssignmentRange,
    ) -> Self {
        Self {
            active_cell,
            assignments,
            interface_ports: None,
            // all continuous assignments are executed under a single control thread
            thread: Some(ThreadMap::continuous_thread()),
            assign_type: AssignType::Continuous,
        }
    }
}
