use crate::ir::{
    self,
    traversal::{Named, Visitor},
    LibrarySignatures,
};
use ir::traversal::{Action, VisResult};

#[derive(Default)]
/// Adds assignments from a components `clk` port to every
/// component that contains an input `clk` port.
pub struct ResetInsertion;

impl Named for ResetInsertion {
    fn name() -> &'static str {
        "reset-insertion"
    }

    fn description() -> &'static str {
        "connect component reset to sub-component reset for applicable components"
    }
}

impl Visitor for ResetInsertion {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        let builder = ir::Builder::new(comp, sigs).generated();

        for cell_ref in &builder.component.cells {
            let cell = cell_ref.borrow();
            if cell.get_attribute("generated").is_some() {
                if let Some(port) = cell.find_with_attr("reset") {
                    builder.component.continuous_assignments.push(
                        builder.build_assignment(
                            port,
                            builder
                                .component
                                .signature
                                .borrow()
                                .get_with_attr("reset"),
                            ir::Guard::True,
                        ),
                    )
                }
            }
        }

        // we don't need to traverse control
        Ok(Action::Stop)
    }
}
