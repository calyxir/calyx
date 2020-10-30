use crate::analysis;
use crate::frontend::library::ast as lib;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
};
use std::collections::HashSet;

#[derive(Default)]
pub struct DeadCellRemoval;

impl Named for DeadCellRemoval {
    fn name() -> &'static str {
        "dead-cell-removal"
    }

    fn description() -> &'static str {
        "removes cells that are never used inside a component"
    }
}

impl Visitor for DeadCellRemoval {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult {
        let mut used_cells: HashSet<ir::Id> = HashSet::new();

        // All cells used in groups
        for group in &comp.groups {
            used_cells.extend(
                &mut analysis::ReadWriteSet::uses(&group.borrow().assignments)
                    .into_iter()
                    .map(|c| c.borrow().name.clone()),
            )
        }

        // All cells used in continuous assignments.
        used_cells.extend(
            &mut analysis::ReadWriteSet::uses(&comp.continuous_assignments)
                .into_iter()
                .map(|c| c.borrow().name.clone()),
        );

        // Remove cells that are not used.
        comp.cells.retain(|c| used_cells.contains(&c.borrow().name));

        Ok(Action::Continue)
    }
}
