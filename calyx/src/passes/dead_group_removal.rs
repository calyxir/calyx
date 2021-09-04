use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    CloneName, LibrarySignatures,
};
use std::collections::HashSet;

/// Removes unused groups and combinational groups from components.
/// A group is considered in use when it shows up in an [ir::Enable].
/// A combinational group is considered in use when it is a part of an
/// [ir::If] or [ir::While].
#[derive(Default)]
pub struct DeadGroupRemoval {
    used_groups: HashSet<ir::Id>,
    used_comb_groups: HashSet<ir::Id>,
}

impl Named for DeadGroupRemoval {
    fn name() -> &'static str {
        "dead-group-removal"
    }

    fn description() -> &'static str {
        "removes unsed groups from components"
    }
}

impl Visitor for DeadGroupRemoval {
    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        self.used_groups.insert(s.group.borrow().clone_name());
        Ok(Action::Continue)
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        if let Some(cg) = &s.cond {
            self.used_comb_groups.insert(cg.borrow().clone_name());
        }
        Ok(Action::Continue)
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        if let Some(cg) = &s.cond {
            self.used_comb_groups.insert(cg.borrow().clone_name());
        }
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        // Remove Groups that are not used
        comp.groups
            .retain(|g| self.used_groups.contains(g.borrow().name()));
        comp.comb_groups
            .retain(|cg| self.used_comb_groups.contains(cg.borrow().name()));

        Ok(Action::Stop)
    }
}
