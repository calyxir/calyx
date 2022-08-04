use super::{GraphAnalysis, ReadWriteSet};
use crate::analysis::ShareSet;
use crate::ir;
use ir::{CloneName, RRC};

/// Detects if a group is solely being used to update a register.
pub struct VariableDetection;

impl VariableDetection {
    /// A group is variable like if it:
    ///  - among write to state_shareable components, there is only one write
    ///  - has `@go` port equal to `1'd1`
    ///  - has `g[done] = cell.done`
    /// Returns the name of the cell if such a group is detected,
    /// otherwise returns `None`.
    pub fn variable_like(
        group_ref: &RRC<ir::Group>,
        state_share: &ShareSet,
    ) -> Option<(ir::CellType, ir::Id)> {
        let group = group_ref.borrow();

        let writes = ReadWriteSet::write_set(group.assignments.iter())
            .filter(|cell| state_share.is_shareable_component(cell))
            .collect::<Vec<_>>();

        if writes.len() != 1 {
            // failed writes check
            return None;
        }

        let cell = writes[0].borrow();

        // check if 1 is being written into go port. This also checks
        // if guard is empty, because if it isn't this would show up as
        // a write
        let graph = GraphAnalysis::from(&*group);
        let go_port = cell.find_with_attr("go")?;
        let activation = graph
            .writes_to(&go_port.borrow())
            .map(|src| src.borrow().is_constant(1, 1))
            .collect::<Vec<_>>();
        if activation.len() != 1 || (!activation.is_empty() && !activation[0]) {
            // failed write_en check
            return None;
        }

        // check to see if `cell.done` is written into `g[done]`
        let activation = graph
            .writes_to(&group.get("done").borrow())
            // Handle g[done] = g ? 1'd1
            .filter(|src| !src.borrow().is_constant(1, 1))
            .map(|src| src.borrow().get_parent_name() == cell.name())
            .collect::<Vec<_>>();
        if activation.len() != 1 || (!activation.is_empty() && !activation[0]) {
            // failed g[done] = reg.done check
            return None;
        }

        Some((cell.prototype.clone(), cell.clone_name()))
    }
}
