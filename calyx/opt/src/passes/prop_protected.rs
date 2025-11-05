use calyx_ir::BoolAttr;

use crate::traversal::{Action, Named, Visitor};

/// Propagates any @protected tags on primitive definitions to all their
/// instances. Used for primitives-level protected annotations
#[derive(Default)]
pub struct PropagateProtected;

impl Named for PropagateProtected {
    fn name() -> &'static str {
        "propagate-protected"
    }

    fn description() -> &'static str {
        "propagates the @protected annotation from primitive definitions to their instances"
    }
}

impl Visitor for PropagateProtected {
    fn start(
        &mut self,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        for cell in comp.cells.iter() {
            let name = if let calyx_ir::CellType::Primitive { name, .. } =
                &cell.borrow().prototype
            {
                *name
            } else {
                continue;
            };

            let prim = sigs.get_primitive(name);
            if prim.attributes.has(BoolAttr::Protected) {
                cell.borrow_mut().add_attribute(BoolAttr::Protected, 1);
            }
        }
        Ok(Action::SkipChildren)
    }
}
