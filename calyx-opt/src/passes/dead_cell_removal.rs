use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir};
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

impl DeadCellRemoval {
    /// Retain the write if the destination is a hole or if the parent of the
    /// destination is read from.
    fn retain_write<T: Clone + Eq + ToString>(
        &self,
        wire_reads: &HashSet<ir::Id>,
        asgn: &ir::Assignment<T>,
    ) -> bool {
        let dst = asgn.dst.borrow();
        if dst.is_hole() {
            true
        } else {
            let parent = &dst.get_parent_name();
            let out =
                self.all_reads.contains(parent) || wire_reads.contains(parent);
            if !out {
                log::debug!(
                    "`{}' because `{}' is unused",
                    ir::Printer::assignment_to_str(asgn),
                    parent
                )
            }
            out
        }
    }

    fn visit_invoke(
        &mut self,
        comp: &ir::RRC<ir::Cell>,
        inputs: &[(ir::Id, ir::RRC<ir::Port>)],
        outputs: &[(ir::Id, ir::RRC<ir::Port>)],
        ref_cells: &[(ir::Id, ir::RRC<ir::Cell>)],
    ) {
        let cells = inputs
            .iter()
            .map(|(_, p)| p)
            .chain(outputs.iter().map(|(_, p)| p))
            .map(|p| p.borrow().get_parent_name())
            .chain(iter::once(comp.borrow().name()))
            .chain(ref_cells.iter().map(|(_, c)| c.borrow().name()));
        self.all_reads.extend(cells);
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

    fn start_static_if(
        &mut self,
        s: &mut ir::StaticIf,
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
        self.visit_invoke(&s.comp, &s.inputs, &s.outputs, &s.ref_cells);
        Ok(Action::Continue)
    }

    fn static_invoke(
        &mut self,
        s: &mut ir::StaticInvoke,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.visit_invoke(&s.comp, &s.inputs, &s.outputs, &s.ref_cells);
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Add @external cells and ref cells.
        self.all_reads.extend(
            comp.cells
                .iter()
                .filter(|c| {
                    let cell = c.borrow();
                    cell.attributes.get(ir::BoolAttr::External).is_some()
                        || cell.is_reference()
                })
                .map(|c| c.borrow().name()),
        );
        // Add component signature
        self.all_reads.insert(comp.signature.borrow().name());

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
            comp.for_each_static_assignment(|assign| {
                assign.for_each_port(|port| {
                    let port = port.borrow();
                    if port.direction == ir::Direction::Output {
                        wire_reads.insert(port.get_parent_name());
                    }
                    None
                });
            });

            // Remove writes to ports on unused cells.
            for gr in comp.get_groups().iter() {
                gr.borrow_mut()
                    .assignments
                    .retain(|asgn| self.retain_write(&wire_reads, asgn))
            }
            // Remove writes to ports on unused cells.
            for gr in comp.get_static_groups().iter() {
                gr.borrow_mut()
                    .assignments
                    .retain(|asgn| self.retain_write(&wire_reads, asgn))
            }
            for cgr in comp.comb_groups.iter() {
                cgr.borrow_mut()
                    .assignments
                    .retain(|asgn| self.retain_write(&wire_reads, asgn))
            }
            comp.continuous_assignments
                .retain(|asgn| self.retain_write(&wire_reads, asgn));

            // Remove unused cells
            let removed = comp.cells.retain(|c| {
                let cell = c.borrow();
                self.all_reads.contains(&cell.name())
                    || wire_reads.contains(&cell.name())
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
