use crate::analysis;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures,
};
use std::collections::HashSet;

/// Removes unused cells from components.
#[derive(Default)]
pub struct DeadCellRemoval {
    used_cells: HashSet<ir::Id>,
}

impl Named for DeadCellRemoval {
    fn name() -> &'static str {
        "dead-cell-removal"
    }

    fn description() -> &'static str {
        "removes cells that are never used inside a component"
    }
}

impl Visitor for DeadCellRemoval {
    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        // add input and output ports to used cells
        self.used_cells.extend(
            s.inputs
                .iter()
                .map(|(_, port)| port.borrow().get_parent_name()),
        );
        self.used_cells.extend(
            s.outputs
                .iter()
                .map(|(_, port)| port.borrow().get_parent_name()),
        );

        self.used_cells.insert(s.comp.borrow().name.clone());

        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        // All cells used in groups
        for group in &comp.groups {
            self.used_cells.extend(
                &mut analysis::ReadWriteSet::uses(&group.borrow().assignments)
                    .into_iter()
                    .map(|c| c.borrow().name.clone()),
            )
        }

        // All cells used in continuous assignments.
        self.used_cells.extend(
            &mut analysis::ReadWriteSet::uses(&comp.continuous_assignments)
                .into_iter()
                .map(|c| c.borrow().name.clone()),
        );

        // Remove cells that are not used.
        comp.cells
            .retain(|c| self.used_cells.contains(&c.borrow().name));

        Ok(Action::Stop)
    }
}
