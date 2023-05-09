use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use std::collections::HashSet;

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
pub struct DataPathInfer;

impl Named for DataPathInfer {
    fn name() -> &'static str {
        "data-path-infer"
    }

    fn description() -> &'static str {
        "Infers @data annotations for cells"
    }
}

impl Visitor for DataPathInfer {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Seed using all cells that have been marked as @control and those that
        // appear in the guard of an assignment
        let mut con_cells: HashSet<ir::Id> = comp
            .cells
            .iter()
            .filter_map(|c| {
                let cell = c.borrow();
                if cell.attributes.has(ir::BoolAttr::Control) {
                    Some(cell.name())
                } else {
                    None
                }
            })
            .collect();

        // If this is used in guards, then it cannot be marked as @data
        comp.for_each_assignment(|asgn| {
            asgn.guard.all_ports().into_iter().for_each(|p| {
                let port = p.borrow();
                if !port.is_hole() {
                    con_cells.insert(port.get_parent_name());
                }
            })
        });
        comp.for_each_static_assignment(|asgn| {
            asgn.guard.all_ports().into_iter().for_each(|p| {
                let port = p.borrow();
                if !port.is_hole() {
                    con_cells.insert(port.get_parent_name());
                }
            })
        });

        // This is a purely structural pass
        Ok(Action::Stop)
    }
}
