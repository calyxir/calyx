use itertools::Itertools;

use crate::errors::{CalyxResult, Error, WithPos};
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{
    self, CloneName, Component, LibrarySignatures, RESERVED_NAMES,
};
use std::collections::HashSet;

/// Pass to check if the program is well-formed.
///
/// Catches the following errors:
/// 1. Programs that don't use a defined group or combinational group.
/// 2. Groups that don't write to their done signal.
/// 3. Groups that write to another group's done signal.
pub struct WellFormed {
    /// Reserved names
    reserved_names: HashSet<String>,
    /// Names of the groups that have been used in the control.
    used_groups: HashSet<ir::Id>,
    /// Names of combinational groups used in the control.
    used_comb_groups: HashSet<ir::Id>,
}

impl Default for WellFormed {
    fn default() -> Self {
        let reserved_names =
            RESERVED_NAMES.iter().map(|s| s.to_string()).collect();

        WellFormed {
            reserved_names,
            used_groups: HashSet::new(),
            used_comb_groups: HashSet::new(),
        }
    }
}

impl Named for WellFormed {
    fn name() -> &'static str {
        "well-formed"
    }

    fn description() -> &'static str {
        "Check if the structure and control are well formed."
    }
}

/// Returns an error if the assignments are obviously conflicting. This happens when two
/// assignments assign to the same port unconditionally.
fn obvious_conflicts<'a, I>(assigns: I) -> CalyxResult<()>
where
    I: Iterator<Item = &'a ir::Assignment>,
{
    let dst_grps = assigns
        .filter(|a| a.guard.is_true())
        .map(|a| (a.dst.borrow().canonical(), a))
        .sorted_by(|(dst1, _), (dst2, _)| ir::Canonical::cmp(dst1, dst2))
        .group_by(|(dst, _)| dst.clone());

    for (_, group) in &dst_grps {
        let assigns = group.map(|(_, a)| a).collect_vec();
        if assigns.len() > 1 {
            let msg = assigns
                .into_iter()
                .map(|a| {
                    a.attributes
                        .copy_span()
                        .map(|s| s.show())
                        .unwrap_or_else(|| ir::Printer::assignment_to_str(a))
                })
                .join("");
            return Err(Error::malformed_structure(format!(
                "Obviously conflicting assignments found:\n{}",
                msg
            )));
        }
    }
    Ok(())
}

impl Visitor for WellFormed {
    fn start(
        &mut self,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Check if any of the cells use a reserved name.
        for cell_ref in comp.cells.iter() {
            let cell = cell_ref.borrow();
            if self.reserved_names.contains(&cell.name().id) {
                return Err(Error::reserved_name(cell.clone_name())
                    .with_pos(cell.name()));
            }
        }

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
                    Err(Error::malformed_structure(
                            format!("Group `{}` refers to the done condition of another group (`{}`).",
                            group.name(),
                            dst.get_parent_name())).with_pos(&dst.attributes))
                } else {
                    Ok(())
                }
            }).collect::<CalyxResult<Vec<_>>>()?;
            if done.is_empty() {
                Err(Error::malformed_structure(format!(
                    "No writes to the `done' hole for group `{gname}'",
                )).with_pos(&group.attributes))
            } else {
                Ok(())
            }
        })?;

        // Check for obvious conflicting assignments
        for gr in comp.groups.iter() {
            obvious_conflicts(
                gr.borrow()
                    .assignments
                    .iter()
                    .chain(comp.continuous_assignments.iter()),
            )?;
        }
        for cgr in comp.comb_groups.iter() {
            obvious_conflicts(
                cgr.borrow()
                    .assignments
                    .iter()
                    .chain(comp.continuous_assignments.iter()),
            )?;
        }
        obvious_conflicts(comp.continuous_assignments.iter())?;

        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.used_groups.insert(s.group.clone_name());

        let group = s.group.borrow();
        let asgn = group.done_cond();
        let const_done_assign =
            asgn.guard.is_true() && asgn.src.borrow().is_constant(1, 1);

        if const_done_assign {
            return Err(Error::malformed_structure("Group with constant done condition is invalid. Use `comb group` instead to define a combinational group.").with_pos(&group.attributes));
        }

        // A group with "static"=0 annotation
        if group
            .attributes
            .get("static")
            .map(|v| *v == 0)
            .unwrap_or(false)
        {
            return Err(Error::malformed_structure("Group with annotation \"static\"=0 is invalid. Use `comb group` instead to define a combinational group or if the group's done condition is not constant, provide the correct \"static\" annotation.").with_pos(&group.attributes));
        }

        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(c) = &s.comb_group {
            self.used_comb_groups.insert(c.clone_name());
        }
        // Only refers to ports defined in the invoked instance.
        let cell = s.comp.borrow();
        let ports: HashSet<_> =
            cell.ports.iter().map(|p| p.borrow().name.clone()).collect();

        s.inputs
            .iter()
            .chain(s.outputs.iter())
            .try_for_each(|(port, _)| {
                if !ports.contains(port) {
                    Err(Error::malformed_structure(format!(
                        "`{}` does not have port named `{}`",
                        cell.name(),
                        port
                    ))
                    .with_pos(&s.attributes))
                } else {
                    Ok(())
                }
            })?;

        Ok(Action::Continue)
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
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
        _comps: &[ir::Component],
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
        _comps: &[ir::Component],
    ) -> VisResult {
        // Go signals of groups mentioned in other groups are considered used
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|pr| {
                let port = pr.borrow();
                if port.is_hole() && port.name == "go" {
                    self.used_groups.insert(port.get_parent_name());
                }
                None
            })
        });

        // Find unused groups
        let all_groups: HashSet<ir::Id> =
            comp.groups.iter().map(|g| g.clone_name()).collect();
        if let Some(group) =
            all_groups.difference(&self.used_groups).into_iter().next()
        {
            let gr = comp.find_group(&group).unwrap();
            let gr = gr.borrow();
            return Err(
                Error::unused(group.clone(), "group").with_pos(&gr.attributes)
            );
        };

        let all_comb_groups: HashSet<ir::Id> =
            comp.comb_groups.iter().map(|g| g.clone_name()).collect();
        if let Some(comb_group) = all_comb_groups
            .difference(&self.used_comb_groups)
            .into_iter()
            .next()
        {
            let cgr = comp.find_comb_group(&comb_group).unwrap();
            let cgr = cgr.borrow();
            return Err(Error::unused(
                comb_group.clone(),
                "combinational group",
            )
            .with_pos(&cgr.attributes));
        }
        Ok(Action::Continue)
    }
}
