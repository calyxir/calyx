use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, GetAttributes, LibrarySignatures};
use calyx_utils::CalyxResult;

/// Turns memory cell primitives with the `@external(1)` attribute into
/// `ref` memory cells without the `@external` attribute.
pub struct ExternalToRef;

impl Named for ExternalToRef {
    fn name() -> &'static str {
        "external-to-ref"
    }

    fn description() -> &'static str {
        "Turn memory cells marked with `@external(1) into `ref` memory cells."
    }
}

impl ConstructVisitor for ExternalToRef {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let external_to_ref = ExternalToRef;
        Ok(external_to_ref)
    }

    fn clear_data(&mut self) {}
}

impl Visitor for ExternalToRef {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Iterate over each cell in the component
        for cell in comp.cells.iter() {
            let mut cell_ref = cell.borrow_mut();
            if cell_ref.get_attributes().has(ir::BoolAttr::External) {
                // Change the cell type to `ref` and remove the external attribute
                cell_ref.get_mut_attributes().remove(ir::BoolAttr::External);
                cell_ref.set_reference(true);
            }
        }
        // Continue visiting other nodes in the AST
        Ok(Action::Continue)
    }
}
