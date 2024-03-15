use crate::flatten::flat_ir::prelude::{GlobalCellIdx, LocalPortOffset};

use super::env::AssignmentRange;

#[derive(Debug)]
pub struct GroupInterfacePorts {
    pub go: LocalPortOffset,
    pub done: LocalPortOffset,
}

/// A group of assignments that is scheduled to be evaluated
#[derive(Debug)]
pub struct ScheduledAssignments {
    pub active_cell: GlobalCellIdx,
    pub assignments: AssignmentRange,
    pub interface_ports: Option<GroupInterfacePorts>,
}

impl ScheduledAssignments {
    pub fn new(
        active_cell: GlobalCellIdx,
        assignments: AssignmentRange,
        interface_ports: Option<GroupInterfacePorts>,
    ) -> Self {
        Self {
            active_cell,
            assignments,
            interface_ports,
        }
    }
}
