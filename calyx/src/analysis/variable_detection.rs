use super::{GraphAnalysis, ReadWriteSet};
use crate::ir;
use ir::RRC;

/// Detects if a group is solely being used to update a register.
pub struct VariableDetection;

impl VariableDetection {
    /// A group is variable like if it:
    ///  - only uses single register
    ///  - has `write_en = 1'd1`
    ///  - has `g[done] = reg.done`
    /// Returns the name of the register if such a group is detected,
    /// otherwise returns `None`.
    pub fn variable_like(group_ref: RRC<ir::Group>) -> Option<ir::Id> {
        let group = group_ref.borrow();

        let writes = ReadWriteSet::write_set(&group.assignments)
            .into_iter()
            .filter(|cell| cell.borrow().type_name() == Some(&"std_reg".into()))
            .collect::<Vec<_>>();

        if writes.len() != 1 {
            // failed writes check
            return None;
        }

        let cell = writes[0].borrow();
        // check if 1 is being written into write_en. This also checks
        // if guard is empty, because if it isn't this would show up as
        // a write
        let graph = GraphAnalysis::from(&*group);
        let activation = graph
            .writes_to(&cell.get("write_en").borrow())
            .map(|src| src.borrow().is_constant(1, 1))
            .collect::<Vec<_>>();
        if activation.len() != 1 || (!activation.is_empty() && !activation[0]) {
            // failed write_en check
            return None;
        }

        // check to see if `reg.done` is written into `g[done]`
        let activation = graph
            .writes_to(&group.get("done").borrow())
            .filter(|src| !src.borrow().is_constant(1, 1))
            .map(|src| src.borrow().get_parent_name() == cell.name())
            .collect::<Vec<_>>();
        if activation.len() != 1 || (!activation.is_empty() && !activation[0]) {
            // failed g[done] = reg.done check
            return None;
        }

        Some(cell.name().clone())
    }
}
