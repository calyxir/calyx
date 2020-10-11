//! Pass to check if the program is well-formed. Catches the following errors:
//! 1. Programs that use reserved SystemVerilog keywords as identifiers.
//! 2. Programs that don't use a defined group.
use crate::errors::Error;
use crate::ir::Component;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use std::collections::HashSet;

pub struct WellFormed {
    /// Set of names that components and cells are not allowed to have.
    reserved_names: HashSet<String>,

    /// Names of the groups that have been used in the control.
    used_groups: HashSet<&'static str>,
}

impl Default for WellFormed {
    fn default() -> Self {
        let reserved_names = vec![
            "reg", "wire", "always", "posedge", "negedge", "logic", "tri",
            "input", "output", "if", "generate", "var", "go", "done", "clk"
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
    fn start(&mut self, comp: &mut Component) -> VisResult {
        // Check if any of the cells use a reserved name.
        for cell_ref in &comp.cells {
            let cell = cell_ref.borrow();
            if self.reserved_names.contains(&cell.name.id) {
                return Err(Error::ReservedName(cell.name.clone()));
            }
        }
        Ok(Action::Continue)
    }
}
