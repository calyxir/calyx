use crate::ir::CloneName;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
};
use std::collections::HashSet;
use std::iter;

/// Removes unused cells from components.
#[derive(Default)]
pub struct DeadCellRemoval {
    /// Names of cells that have been read from.
    all_reads: HashSet<ir::Id>,
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
    fn start_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.all_reads.insert(s.port.borrow().get_parent_name());
        Ok(Action::Continue)
    }

    fn start_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.all_reads.insert(s.port.borrow().get_parent_name());
        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let ir::Invoke {
            inputs, outputs, ..
        } = s;
        let cells = inputs
            .iter()
            .map(|(_, p)| p)
            .chain(outputs.iter().map(|(_, p)| p))
            .map(|p| p.borrow().get_parent_name())
            .chain(iter::once(s.comp.clone_name()));
        self.all_reads.extend(cells);
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Add all combinational cells that have at least one output read.
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                let port = port.borrow();
                if port.direction == ir::Direction::Output {
                    self.all_reads.insert(port.get_parent_name());
                }
                None
            });
        });

        // Remove writes to ports on unused cells.
        for gr in comp.groups.iter() {
            gr.borrow_mut().assignments.retain(|asgn| {
                let dst = asgn.dst.borrow();
                dst.is_hole() || self.all_reads.contains(&dst.get_parent_name())
            })
        }
        for cgr in comp.comb_groups.iter() {
            cgr.borrow_mut().assignments.retain(|asgn| {
                let dst = asgn.dst.borrow();
                self.all_reads.contains(&dst.get_parent_name())
            })
        }
        comp.continuous_assignments.retain(|asgn| {
            let dst = asgn.dst.borrow();
            dst.is_hole() || self.all_reads.contains(&dst.get_parent_name())
        });

        // Remove unused cells
        comp.cells
            .retain(|c| self.all_reads.contains(c.borrow().name()));

        Ok(Action::Stop)
    }
}
