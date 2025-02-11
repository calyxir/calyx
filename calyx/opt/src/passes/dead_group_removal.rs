use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use std::collections::HashSet;

/// Removes unused groups and combinational groups from components.
/// A group is considered in use when it shows up in an [ir::Enable].
/// A combinational group is considered in use when it is a part of an
/// [ir::If] or [ir::While] or [ir::Invoke].
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
        _comps: &[ir::Component],
    ) -> VisResult {
        self.used_groups.insert(s.group.borrow().name());
        Ok(Action::Continue)
    }

    fn static_enable(
        &mut self,
        s: &mut ir::StaticEnable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.used_groups.insert(s.group.borrow().name());
        Ok(Action::Continue)
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(cg) = &s.cond {
            self.used_comb_groups.insert(cg.borrow().name());
        }
        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(cg) = &s.comb_group {
            self.used_comb_groups.insert(cg.borrow().name());
        }
        Ok(Action::Continue)
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(cg) = &s.cond {
            self.used_comb_groups.insert(cg.borrow().name());
        }
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Groups that are driven by their `go` signals should not be
        // removed.
        for group in comp.get_groups().iter() {
            for assign in &group.borrow().assignments {
                let dst = assign.dst.borrow();
                if dst.is_hole() && dst.name == "go" {
                    self.used_groups.insert(dst.get_parent_name());
                }
            }
        }

        for assign in &comp.continuous_assignments {
            let dst = assign.dst.borrow();
            if dst.is_hole() && dst.name == "go" {
                self.used_groups.insert(dst.get_parent_name());
            }
        }

        for group in comp.get_static_groups().iter() {
            for assign in &group.borrow().assignments {
                let dst = assign.dst.borrow();
                if dst.is_hole() && dst.name == "go" {
                    self.used_groups.insert(dst.get_parent_name());
                }
            }
        }

        // Remove Groups that are not used
        comp.get_groups_mut()
            .retain(|g| self.used_groups.contains(&g.borrow().name()));
        comp.get_static_groups_mut()
            .retain(|g| self.used_groups.contains(&g.borrow().name()));
        comp.comb_groups
            .retain(|cg| self.used_comb_groups.contains(&cg.borrow().name()));

        Ok(Action::Stop)
    }
}
