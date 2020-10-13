//! Compiles away all `empty` statements in a FuTIL program to a group that is
//! always active.
use crate::{structure, build_assignments};
use crate::frontend::library::ast::LibrarySignatures;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Component, Control};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
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
    fn finish_empty(
        &mut self,
        _s: &ir::Empty,
        comp: &mut Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        let group_ref = match comp.find_group(&CompileEmpty::EMPTY_GROUP.into())
        {
            Some(g) => g,
            None => {
                let mut builder = ir::Builder::from(comp, sigs, false);
                // Create a group that always outputs done if it doesn't exist.
                let mut attrs = HashMap::new();
                attrs.insert("static".to_string(), 0);

                // Add the new group
                let empty_group = builder
                    .add_group(CompileEmpty::EMPTY_GROUP.to_string(), attrs);

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

        return Ok(Action::Change(Control::enable(Rc::clone(&group_ref))));
    }
}
