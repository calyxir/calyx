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
        let group_ref = match comp.find_group(CompileEmpty::EMPTY_GROUP.into())
        {
            Some(g) => g,
            None => {
                // Create a group that always outputs done if it doesn't exist.
                let mut attrs = HashMap::new();
                attrs.insert("static".to_string(), 0);

                // Create a new group
                let empty_group = comp
                    .build_group(CompileEmpty::EMPTY_GROUP.to_string(), attrs);
                comp.groups.push(Rc::clone(&empty_group));

                // Add this signal empty_group[done] = 1'd1;
                let signal_on = comp.build_constant(1, 1);
                let done_assign = comp.build_assignment(
                    empty_group.borrow().find_hole("done".into()).unwrap(),
                    signal_on.borrow().find_port("out".into()).unwrap(),
                    None,
                );
                empty_group.borrow_mut().assignments.push(done_assign);
                empty_group
            }
        };

        return Ok(Action::Change(Control::enable(Rc::clone(&group_ref))));
    }
}
