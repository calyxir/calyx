use itertools::Itertools;

use crate::analysis::ReadWriteSet;
use crate::ir::{
    self,
    traversal::{Action, Loggable, Named, VisResult, Visitor},
};

/// Transform groups that are structurally invoking components into equivalent
/// [ir::Invoke] statements.
///
/// For a group to meet the requirements of this pass, it must
/// 1. Only use unguarded assignments
/// 2. Only assign to input ports of one component
/// 3. Assign `1'd1` to the @go port of the component, and
/// 4. Depend directly on the @done port of the component for its done
///    condition.
#[derive(Default)]
pub struct GroupToInvoke;

impl Named for GroupToInvoke {
    fn name() -> &'static str {
        "group2invoke"
    }

    fn description() -> &'static str {
        "covert groups that structurally invoke one component into invoke statements"
    }
}

impl Visitor for GroupToInvoke {
    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        let group = s.group.borrow();

        // There should be exactly one component being written to in the
        // group.
        let writes = ReadWriteSet::write_set(&group.assignments).collect_vec();
        if writes.len() != 1 {
            return Ok(Action::Continue);
        }

        // Component must define a @go/@done interface
        let cell = writes[0].borrow();
        let maybe_go_port = cell.find_with_attr("go");
        let maybe_done_port = cell.find_with_attr("done");
        if maybe_go_port.is_none() || maybe_done_port.is_none() {
            return Ok(Action::Continue);
        }

        let go_port = maybe_go_port.unwrap();
        let mut go_multi_write = false;
        let done_port = maybe_done_port.unwrap();
        let mut done_multi_write = false;
        for assign in &group.assignments {
            // All assignments should be unguaraded.
            if !assign.guard.is_true() {
                return Ok(Action::Continue);
            }
            // @go port should have exactly one write and the src should be 1.
            if assign.dst == go_port {
                if go_multi_write {
                    return Ok(Action::Continue);
                }
                if !go_multi_write && assign.src.borrow().is_constant(1, 1) {
                    go_multi_write = true;
                }
            }
            // @done port should have exactly one read and the dst should be
            // group's done signal.
            if assign.src == done_port {
                if done_multi_write {
                    return Ok(Action::Continue);
                }
                if !done_multi_write && assign.dst == group.get("done") {
                    done_multi_write = true;
                }
            }
        }

        self.elog("check", group.name());

        Ok(Action::Continue)
    }
}
