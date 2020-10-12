//! Compiles away all `empty` statements in a FuTIL program to a group that is
//! always active.
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
    ) -> VisResult {
        let group_ref = match comp.find_group(&CompileEmpty::EMPTY_GROUP.into())
        {
            Some(g) => g,
            None => {
                let mut builder = ir::Builder::from(comp, false);
                // Create a group that always outputs done if it doesn't exist.
                let mut attrs = HashMap::new();
                attrs.insert("static".to_string(), 0);

                // Add the new group
                let empty_group = builder
                    .add_group(CompileEmpty::EMPTY_GROUP.to_string(), attrs);

                // Add this signal empty_group[done] = 1'd1;
                let signal_on = builder.add_constant(1, 1);
                let done_assign = builder.build_assignment(
                    empty_group.borrow().get_hole(&"done".into()),
                    signal_on.borrow().get_port(&"out".into()),
                    None,
                );
                empty_group.borrow_mut().assignments.push(done_assign);
                empty_group
            }
        };

        return Ok(Action::Change(Control::enable(Rc::clone(&group_ref))));
    }
}
