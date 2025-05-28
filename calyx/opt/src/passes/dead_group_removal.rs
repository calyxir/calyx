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

impl DeadGroupRemoval {
    /// A function that in-place updates a Vec with the name of the parent
    /// of a port, if that port parent is a Group
    fn push_group_names(
        group_names: &mut Vec<ir::Id>,
        port: &ir::RRC<ir::Port>,
    ) {
        if let ir::PortParent::Group(group_wref) = &port.borrow().parent {
            group_names.push(group_wref.upgrade().borrow().name());
        }
    }
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

    fn fsm_enable(
        &mut self,
        s: &mut calyx_ir::FSMEnable,
        _comp: &mut calyx_ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        // add all groups that are assigned to / read from, by the parent FSM
        self.used_groups.extend(
            s.fsm
                .borrow()
                .get_called_port_parents(DeadGroupRemoval::push_group_names),
        );
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

        // add all groups invoked by each fsm
        for fsm in comp.get_fsms().iter() {
            self.used_groups.extend(
                fsm.borrow().get_called_port_parents(
                    DeadGroupRemoval::push_group_names,
                ),
            );
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
