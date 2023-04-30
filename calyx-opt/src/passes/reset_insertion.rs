use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};

#[derive(Default)]
/// Adds assignments from a components `reset` port to every
/// component that contains an input `reset` port.
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
        _comps: &[ir::Component],
    ) -> VisResult {
        let builder = ir::Builder::new(comp, sigs);

        for cell_ref in builder.component.cells.iter() {
            let cell = cell_ref.borrow();
            if cell.get_attribute(ir::Attribute::External).is_some() {
                // External cells should not have their state reset,
                // since we assume they may be initialized.
                continue;
            }
            if let Some(port) = cell.find_with_attr(ir::Attribute::Reset) {
                builder.component.continuous_assignments.push(
                    builder.build_assignment(
                        port,
                        builder
                            .component
                            .signature
                            .borrow()
                            .get_with_attr(ir::Attribute::Reset),
                        ir::Guard::True,
                    ),
                )
            }
        }

        // we don't need to traverse control
        Ok(Action::Stop)
    }
}
