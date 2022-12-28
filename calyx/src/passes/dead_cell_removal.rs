use crate::ir::CloneName;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
};
use std::collections::HashSet;
use std::iter;

/// Warn if dead cell removal loops more than this number of times
const LOOP_THRESHOLD: u64 = 5;

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
            .chain(iter::once(s.comp.clone_name()))
            .chain(s.ref_cells.iter().map(|(_, cell)| cell.clone_name()));
        self.all_reads.extend(cells);
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Add @external cells.
        self.all_reads.extend(
            comp.cells
                .iter()
                .filter(|c| {
                    c.borrow().is_reference()
                        || c.borrow().attributes.get("external").is_some()
                })
                .map(|c| c.clone_name()),
        );
        // Add component signature
        self.all_reads.insert(comp.signature.clone_name());

        // Add all cells that have at least one output read.
        let mut count = 0;
        loop {
            let mut wire_reads = HashSet::new();
            comp.for_each_assignment(|assign| {
                assign.for_each_port(|port| {
                    let port = port.borrow();
                    if port.direction == ir::Direction::Output {
                        wire_reads.insert(port.get_parent_name());
                    }
                    None
                });
            });

            // Remove writes to ports on unused cells.
            for gr in comp.groups.iter() {
                gr.borrow_mut().assignments.retain(|asgn| {
                    let dst = asgn.dst.borrow();
                    if dst.is_hole() {
                        true
                    } else {
                        let parent = &dst.get_parent_name();
                        self.all_reads.contains(parent)
                            || wire_reads.contains(parent)
                    }
                })
            }
            for cgr in comp.comb_groups.iter() {
                cgr.borrow_mut().assignments.retain(|asgn| {
                    let dst = asgn.dst.borrow();
                    let parent = &dst.get_parent_name();
                    self.all_reads.contains(parent)
                        || wire_reads.contains(parent)
                })
            }
            comp.continuous_assignments.retain(|asgn| {
                let dst = asgn.dst.borrow();
                if dst.is_hole() {
                    true
                } else {
                    let parent = &dst.get_parent_name();
                    self.all_reads.contains(parent)
                        || wire_reads.contains(parent)
                }
            });

            // Remove unused cells
            let removed = comp.cells.retain(|c| {
                let cell = c.borrow();
                let out = self.all_reads.contains(&cell.name())
                    || wire_reads.contains(&cell.name());
                if !out {
                    log::debug!("Unused cell {}", cell.name());
                }
                out
            });

            if removed == 0 {
                break;
            }

            count += 1;
        }

        if count >= LOOP_THRESHOLD {
            log::warn!("{} looped {count} times", Self::name());
        }

        Ok(Action::Stop)
    }
}
