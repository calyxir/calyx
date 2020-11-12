use crate::errors::Error;
use crate::frontend::library::ast::LibrarySignatures;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Component};
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

    /// All of the groups used in the program.
    all_groups: HashSet<ir::Id>,
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
            all_groups: HashSet::new(),
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

impl Visitor<()> for WellFormed {
    fn start(
        &mut self,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult<()> {
        for group_ref in &comp.groups {
            self.all_groups.insert(group_ref.borrow().name.clone());
        }
        // Check if any of the cells use a reserved name.
        for cell_ref in &comp.cells {
            let cell = cell_ref.borrow();
            if self.reserved_names.contains(&cell.name.id) {
                return Err(Error::ReservedName(cell.name.clone()));
            }
            if self.all_groups.contains(&cell.name) {
                return Err(Error::AlreadyBound(
                    cell.name.clone(),
                    "group".to_string(),
                ));
            }
        }
        Ok(Action::continue_default())
    }

    fn start_enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult<()> {
        self.used_groups.insert(s.group.borrow().name.clone());
        Ok(Action::continue_default())
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult<()> {
        // Add cond group as a used port.
        self.used_groups.insert(s.cond.borrow().name.clone());
        Ok(Action::continue_default())
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult<()> {
        // Add cond group as a used port.
        self.used_groups.insert(s.cond.borrow().name.clone());
        Ok(Action::continue_default())
    }

    fn finish(
        &mut self,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult<()> {
        let unused_group = self
            .all_groups
            .difference(&self.used_groups)
            .into_iter()
            .next();
        match unused_group {
            Some(group) => Err(Error::UnusedGroup(group.clone())),
            None => Ok(Action::continue_default()),
        }
    }
}
