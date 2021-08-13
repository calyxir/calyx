use crate::errors::Error;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, CloneName, Component, LibrarySignatures};
use std::collections::HashSet;

/// Pass to check if the program is well-formed.
///
/// Catches the following errors:
/// 1. Programs that use reserved SystemVerilog keywords as identifiers.
/// 2. Programs that don't use a defined group.
pub struct WellFormed {
    /// Set of names that components and cells are not allowed to have.
    reserved_names: HashSet<String>,

    /// Names of the groups that have been used in the control.
    used_groups: HashSet<ir::Id>,
}

impl Default for WellFormed {
    fn default() -> Self {
        let reserved_names = vec![
            "reg", "wire", "always", "posedge", "negedge", "logic", "tri",
            "input", "output", "if", "generate", "var", "go", "done", "clk",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        WellFormed {
            reserved_names,
            used_groups: HashSet::new(),
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

impl Visitor for WellFormed {
    fn start(
        &mut self,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        // Check if any of the cells use a reserved name.
        for cell_ref in comp.cells.iter() {
            let cell = cell_ref.borrow();
            if self.reserved_names.contains(&cell.name().id) {
                return Err(Error::ReservedName(cell.clone_name()));
            }
        }

        // For each group, check if there is at least one write to the done
        // signal of that group.
        // Names of the groups whose `done` hole has been written to.
        for group_ref in comp.groups.iter() {
            let group = group_ref.borrow();
            let gname = group.name();
            // Find an assignment writing to this group's done condition.
            let done = group.assignments.iter().find(|assign| {
                let dst = assign.dst.borrow();
                dst.is_hole()
                    && dst.name == "done"
                    && dst.get_parent_name() == gname
            });
            if done.is_none() {
                return Err(Error::MalformedStructure(gname.fmt_err(
                    &format!(
                        "No writes to the `done' hole for group `{}'",
                        gname.to_string()
                    ),
                )));
            }
        }

        // Check if any groups refer to another group's done signal.
        for group_ref in comp.groups.iter() {
            let group = group_ref.borrow();
            for assign in &group.assignments {
                let dst = assign.dst.borrow();
                if dst.is_hole()
                    && dst.name == "done"
                    && *group.name() != dst.get_parent_name()
                {
                    return Err(
                        Error::MalformedStructure(
                            format!("Group `{}` refers to the done condition of another group (`{}`).",
                                group.name(),
                                dst.get_parent_name())));
                }
            }
        }

        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        self.used_groups.insert(s.group.clone_name());
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
        Ok(Action::Continue)
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        // Add cond group as a used port.
        self.used_groups.insert(s.cond.clone_name());
        Ok(Action::Continue)
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        // Add cond group as a used port.
        self.used_groups.insert(s.cond.clone_name());
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        let all_groups: HashSet<ir::Id> =
            comp.groups.iter().map(|g| g.clone_name()).collect();
        let unused_group =
            all_groups.difference(&self.used_groups).into_iter().next();
        match unused_group {
            Some(group) => Err(Error::UnusedGroup(group.clone())),
            None => Ok(Action::Continue),
        }
    }
}
