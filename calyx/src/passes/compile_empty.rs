use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Component, Control, LibrarySignatures};
use crate::{build_assignments, structure};
use std::rc::Rc;

#[derive(Default)]
/// Compiles away all [`ir::Empty`](crate::ir::Empty) statements into an
/// [`ir::Enable`](crate::ir::Enable).
///
/// The generated program enables the following group:
/// ```calyx
/// cells {
///     @generated empty_reg = std_reg(1);
/// }
///
/// group _empty {
///     empty_reg.write_en = 1'd1;
///     empty_reg.in = 1'd1;
///     _empty["done"] = empty_reg.done;
/// }
/// ```
pub struct CompileEmpty {
    // Name of the empty group if it has been created.
    group_name: Option<ir::Id>,
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
        let group_ref = match &self.group_name {
            Some(g) => comp.find_group(g).unwrap(),
            None => {
                let mut builder = ir::Builder::new(comp, sigs);

                // Build the new group
                let empty_group = builder.add_group("_empty");
                empty_group.borrow_mut().attributes.insert("static", 1);

                // Add this signal empty_group[done] = 1'd1;
                structure!(builder;
                    let signal_on = constant(1, 1);
                    let empty_reg = prim std_reg(1);
                );
                let mut assigns: Vec<_> = build_assignments!(builder;
                    empty_reg["write_en"] = ? signal_on["out"];
                    empty_reg["in"] = ? signal_on["out"];
                    empty_group["done"] = ? empty_reg["done"];
                );
                empty_group.borrow_mut().assignments.append(&mut assigns);

                // Register the name of the group to the pass
                self.group_name = Some(empty_group.borrow().name().clone());

                empty_group
            }
        };

        Ok(Action::Change(Control::enable(Rc::clone(&group_ref))))
    }

    fn finish(
        &mut self,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        // The empty group, if created, is only defined for this component.
        // Deregister it before walking over another group.
        self.group_name = None;
        Ok(Action::Continue)
    }
}
