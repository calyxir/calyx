use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::Error;
use std::{collections::HashSet, hash::Hash, rc::Rc};

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
        let mut used_reset = HashSet::new();
        comp.fsms.iter().for_each(|fsm| {
            fsm.borrow().assignments.iter().for_each(|assigns| {
                assigns.iter().for_each(|assign| {
                    if assign.dst.borrow().name == "reset" {
                        used_reset
                            .insert(assign.dst.borrow().get_parent_name());
                    }
                })
            })
        });
        let builder = ir::Builder::new(comp, sigs);
        let reset = builder
            .component
            .signature
            .borrow()
            .find_unique_with_attr(ir::BoolAttr::Reset)?;

        if let Some(reset) = reset {
            for cell_ref in builder.component.cells.iter() {
                let cell = cell_ref.borrow();
                if let Some(port) =
                    cell.find_unique_with_attr(ir::BoolAttr::Reset)?
                {
                    if !used_reset.contains(&port.borrow().get_parent_name()) {
                        builder.component.continuous_assignments.push(
                            builder.build_assignment(
                                port,
                                Rc::clone(&reset),
                                ir::Guard::True,
                            ),
                        )
                    }
                }
            }
        } else {
            for cell_ref in builder.component.cells.iter() {
                let cell = cell_ref.borrow();
                if cell.find_unique_with_attr(ir::BoolAttr::Reset)?.is_some() {
                    return Err(Error::malformed_structure(format!(
                        "Cell `{}' in component `{}' has a reset port, \
                        but the component does not have a reset port.",
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
