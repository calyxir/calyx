use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use ir::RRC;
use std::{collections::HashSet, rc::Rc};

#[derive(Default)]
/// Infers `@control` and `@data` annotations for cells.
/// A cell marked with `@data` can have `'x` assignments to its `@data` ports
/// which enables downstream optimizations.
///
/// A cell cannot be marked `@data` iff:
/// * If it is used in the guard of an assignment
/// * If it is used as the done condition of a group
/// * If it is used as the conditional port for if or while
/// * If it is used as the input to a @go port such as write_en
/// * If it is used as an input for another @control instance
///
/// Because the last constraint is recursive, we use an iterative algorithm to
/// infer the annotations.
pub struct DataPathInfer {
    /// Cells that cannot be marked as a @data cell
    control_cells: HashSet<ir::Id>,
}

impl Named for DataPathInfer {
    fn name() -> &'static str {
        "data-path-infer"
    }

    fn description() -> &'static str {
        "Infers @data annotations for cells"
    }
}

impl DataPathInfer {
    fn port_and_cg(
        &mut self,
        port: RRC<ir::Port>,
        mb_cg: &Option<RRC<ir::CombGroup>>,
    ) {
        let cond_port = port.borrow();
        assert!(!cond_port.is_hole());
        self.control_cells.insert(cond_port.get_parent_name());

        // All ports used in the combinational group cannot be data ports
        // since they are used to compute the condition.
        if let Some(cgr) = mb_cg {
            let cg = cgr.borrow();
            for assign in &cg.assignments {
                let src = assign.src.borrow();
                assert!(!src.is_hole());
                self.control_cells.insert(src.get_parent_name());
            }
        }
    }
}

impl Visitor for DataPathInfer {
    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.port_and_cg(Rc::clone(&s.port), &s.cond);
        Ok(Action::Continue)
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.port_and_cg(Rc::clone(&s.port), &s.cond);
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Seed using all cells that have been marked as @control and those that
        // appear in the guard of an assignment
        self.control_cells.extend(comp.cells.iter().filter_map(|c| {
            let cell = c.borrow();
            if cell.attributes.has(ir::BoolAttr::Control) {
                Some(cell.name())
            } else {
                None
            }
        }));

        // If this is used in guards, then it cannot be marked as @data
        comp.for_each_assignment(|asgn| {
            asgn.guard.all_ports().into_iter().for_each(|p| {
                let port = p.borrow();
                if !port.is_hole() {
                    self.control_cells.insert(port.get_parent_name());
                }
            });
            // If this assignment writes to the done condition of a group, then the source
            // cannot be marked as @data
            if asgn.dst.borrow().is_hole() {
                self.control_cells
                    .insert(asgn.src.borrow().get_parent_name());
            }
        });
        comp.for_each_static_assignment(|asgn| {
            asgn.guard.all_ports().into_iter().for_each(|p| {
                let port = p.borrow();
                if !port.is_hole() {
                    self.control_cells.insert(port.get_parent_name());
                }
            })
        });

        // Mark all control cells
        for c in comp.cells.iter() {
            let mut cell = c.borrow_mut();
            if self.control_cells.contains(&cell.name()) {
                cell.attributes.insert(ir::BoolAttr::Control, 1);
            }
        }

        Ok(Action::Stop)
    }
}
