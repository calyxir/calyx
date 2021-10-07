use crate::errors::{CalyxResult, Error};
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, CloneName, Component, LibrarySignatures};
use std::collections::HashSet;

/// Pass to check if the program is well-formed.
///
/// Catches the following errors:
/// 1. Programs that don't use a defined group or combinational group.
/// 2. Groups that don't write to their done signal.
/// 3. Groups that write to another group's done signal.
#[derive(Default)]
pub struct WellFormed {
    /// Names of the groups that have been used in the control.
    used_groups: HashSet<ir::Id>,
    /// Names of combinational groups used in the control.
    used_comb_groups: HashSet<ir::Id>,
}

impl Named for WellFormed {
    fn name() -> &'static str {
        "well-formed"
    }

    fn description() -> &'static str {
        "Check if the structure and control are well formed."
    }
}

impl Visitor for WellFormed {
    fn start(
        &mut self,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        // For each non-combinational group, check if there is at least one write to the done
        // signal of that group and that the write is to the group's done signal.
        comp.groups.iter().try_for_each(|group_ref| {
            let group = group_ref.borrow();
            let gname = group.name();
            // Find an assignment writing to this group's done condition.
            let done = group.assignments.iter().filter(|assign| {
                let dst = assign.dst.borrow();
                dst.is_hole()
                    && dst.name == "done"
            }).map(|assign| {
                let dst = assign.dst.borrow();
                if gname != &dst.get_parent_name() {
                    Err(Error::MalformedStructure(
                            format!("Group `{}` refers to the done condition of another group (`{}`).",
                            group.name(),
                            dst.get_parent_name())))
                } else {
                    Ok(())
                }
            }).collect::<CalyxResult<Vec<_>>>()?;
            if done.is_empty() {
                Err(Error::MalformedStructure(gname.fmt_err(&format!(
                    "No writes to the `done' hole for group `{}'",
                    gname.to_string()
                ))))
            } else {
                Ok(())
            }
        })?;

        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        self.used_groups.insert(s.group.clone_name());

        let group = s.group.borrow();
        let done_assign = group
            .assignments
            .iter()
            .find(|assign| {
                let dst = assign.dst.borrow();
                dst.is_hole() && *group.name() == dst.get_parent_name()
            })
            .map(|asgn| {
                asgn.guard.is_true() && asgn.src.borrow().is_constant(1, 1)
            });

        // A group with a constant done condition are not allowed.
        if group
            .attributes
            .get("static")
            .map(|v| *v == 0)
            .unwrap_or(false)
            || done_assign.unwrap_or(false)
        {
            return Err(Error::MalformedStructure(group.name().fmt_err("Group with constant done condition are invalid. Use `comb group` instead to define a combinational group.")));
        }

        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        for (id, port) in &s.inputs {
            if port.borrow().direction != ir::Direction::Output {
                panic!(
                    "Input argument `{}` for `invoke {}` uses non-output port: `{}`. Input arguments should use output ports.",
                    id,
                    s.comp.borrow().name(),
                    port.borrow().name)
            }
        }
        for (id, port) in &s.outputs {
            if port.borrow().direction != ir::Direction::Input {
                panic!(
                    "Output argument `{}` for `invoke {}` uses non-input port: `{}`. Output arguments should use input ports.",
                    id,
                    s.comp.borrow().name(),
                    port.borrow().name)
            }
        }
        // Add cond group as a used port.
        if let Some(c) = &s.comb_group {
            self.used_comb_groups.insert(c.clone_name());
        }
        Ok(Action::Continue)
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        // Add cond group as a used port.
        if let Some(cond) = &s.cond {
            self.used_comb_groups.insert(cond.clone_name());
        }
        Ok(Action::Continue)
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        // Add cond group as a used port.
        if let Some(cond) = &s.cond {
            self.used_comb_groups.insert(cond.clone_name());
        }
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        let all_groups: HashSet<ir::Id> =
            comp.groups.iter().map(|g| g.clone_name()).collect();
        if let Some(group) =
            all_groups.difference(&self.used_groups).into_iter().next()
        {
            return Err(Error::UnusedGroup(group.clone()));
        };

        let all_comb_groups: HashSet<ir::Id> =
            comp.comb_groups.iter().map(|g| g.clone_name()).collect();
        if let Some(group) = all_comb_groups
            .difference(&self.used_comb_groups)
            .into_iter()
            .next()
        {
            return Err(Error::UnusedGroup(group.clone()));
        }
        Ok(Action::Continue)
    }
}
