use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Component, Control, LibrarySignatures};
use crate::{build_assignments, structure};
use std::rc::Rc;

#[derive(Default)]
/// Compiles away all [`ir::Empty`](crate::ir::Empty) statements into an
/// [`ir::Enable`](crate::ir::Enable).
///
/// The generated program enables the following group:
/// ```
/// group _empty {
///     _empty["done"] = 1'd1;
/// }
/// ```
pub struct CompileEmpty {}

impl CompileEmpty {
    const EMPTY_GROUP: &'static str = "_empty";
}

impl Named for CompileEmpty {
    fn name() -> &'static str {
        "compile-empty"
    }

    fn description() -> &'static str {
        "Rewrites empty control to invocation to empty group"
    }
}

impl Visitor for CompileEmpty {
    fn empty(
        &mut self,
        _s: &mut ir::Empty,
        comp: &mut Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        let group_ref = match comp.find_group(&CompileEmpty::EMPTY_GROUP) {
            Some(g) => g,
            None => {
                let mut builder = ir::Builder::from(comp, sigs, false);
                // Create a group that always outputs done if it doesn't exist.

                // Add the new group
                let empty_group =
                    builder.add_group(CompileEmpty::EMPTY_GROUP.to_string());
                empty_group.borrow_mut().attributes.insert("static", 0);

                // Add this signal empty_group[done] = 1'd1;
                structure!(builder;
                    let signal_on = constant(1, 1);
                );
                let mut assigns: Vec<_> = build_assignments!(builder;
                    empty_group["done"] = ? signal_on["out"];
                );
                empty_group.borrow_mut().assignments.append(&mut assigns);
                empty_group
            }
        };

        Ok(Action::Change(Control::enable(Rc::clone(&group_ref))))
    }
}
