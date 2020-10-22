//! Adds assignments from a components `clk` port to every
//! component that contains an input `clk` port. For example

use crate::frontend::library::ast as lib;
use crate::ir::{
    self,
    traversal::{Named, Visitor},
};
use ir::traversal::{Action, VisResult};

#[derive(Default)]
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
        sigs: &lib::LibrarySignatures,
    ) -> VisResult {
        let builder = ir::Builder::from(comp, sigs, false);

        for cell_ref in &builder.component.cells {
            let cell = cell_ref.borrow();
            match cell.find("clk") {
                Some(port) => builder.component.continuous_assignments.push(
                    builder.build_assignment(
                        port,
                        builder.component.signature.borrow().get("clk"),
                        ir::Guard::True,
                    ),
                ),
                None => (),
            }
        }

        // we don't need to traverse control
        Ok(Action::Stop)
    }
}
