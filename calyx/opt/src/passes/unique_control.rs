use std::collections::HashMap;

use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{
    self as ir, BoolAttr, Guard, Id, Nothing, NumAttr, StaticTiming,
};
use calyx_utils::CalyxResult;

/// Adds probe wires to each group (includes static groups and comb groups) to detect when a group is active.
/// Used by the profiler.
pub struct UniqueControl {}

impl Named for UniqueControl {
    fn name() -> &'static str {
        "unique-control"
    }

    fn description() -> &'static str {
        "Make all control enables unique by adding a wrapper group"
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![]
    }
}

impl ConstructVisitor for UniqueControl {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        Ok(UniqueControl {})
    }

    fn clear_data(&mut self) {}
}

impl Visitor for UniqueControl {
    // fn start(
    //     &mut self,
    //     comp: &mut ir::Component,
    //     sigs: &ir::LibrarySignatures,
    //     _comps: &[ir::Component],
    // ) -> VisResult {

    //     Ok(Action::Continue)
    // }

    fn enable(
        &mut self,
        s: &mut calyx_ir::Enable,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let group_name = s.group.borrow().name();
        // create a wrapper group
        let mut builder = ir::Builder::new(comp, sigs);
        let unique_group = builder.add_group(group_name);
        // let unique_group_assignments = s.group.borrow().assignments.clone();
        let mut unique_group_assignments: Vec<calyx_ir::Assignment<Nothing>> =
            Vec::new();
        for asgn in s.group.borrow().assignments.iter() {
            if asgn.dst.borrow().get_parent_name() == group_name
                && asgn.dst.borrow().name == "done"
            {
                // done needs to be reassigned
                let new_done_asgn = builder.build_assignment(
                    unique_group.borrow().get("done"),
                    asgn.src.clone(),
                    *asgn.guard.clone(),
                );
                unique_group_assignments.push(new_done_asgn);
            } else {
                unique_group_assignments.push(asgn.clone());
            }
        }
        unique_group
            .borrow_mut()
            .assignments
            .append(&mut unique_group_assignments);
        Ok(Action::Change(Box::new(ir::Control::enable(unique_group))))
    }
}
