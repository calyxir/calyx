use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::Error;
use std::rc::Rc;

#[derive(Default)]
/// Adds assignments from a components `clk` port to every
/// component that contains an input `clk` port.
pub struct ClkInsertion;

impl Named for ClkInsertion {
    fn name() -> &'static str {
        "clk-insertion"
    }

    fn description() -> &'static str {
        "inserts assignments from component clk to sub-component clk"
    }
}

impl Visitor for ClkInsertion {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let builder = ir::Builder::new(comp, sigs);
        // Find @clk port in the component. If it doesn't exist,
        // then we don't need to do anything.
        let clk = builder
            .component
            .signature
            .borrow()
            .find_unique_with_attr(ir::BoolAttr::Clk)?;

        if let Some(clk) = clk {
            for cell_ref in builder.component.cells.iter() {
                let cell = cell_ref.borrow();
                if let Some(port) =
                    cell.find_unique_with_attr(ir::BoolAttr::Clk)?
                {
                    builder.component.continuous_assignments.push(
                        builder.build_assignment(
                            port,
                            Rc::clone(&clk),
                            ir::Guard::True,
                        ),
                    )
                }
            }
        } else {
            for cell_ref in builder.component.cells.iter() {
                let cell = cell_ref.borrow();
                if cell.find_unique_with_attr(ir::BoolAttr::Clk)?.is_some() {
                    return Err(Error::malformed_structure(format!(
                        "Cell `{}' in component `{}' has a clk port, \
                        but the component does not have a clk port.",
                        cell.name(),
                        builder.component.name
                    )));
                }
            }
        }

        // we don't need to traverse control
        Ok(Action::Stop)
    }
}
