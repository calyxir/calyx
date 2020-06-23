use crate::lang::component::Component;
use crate::lang::{ast, context::Context, structure_iter::ConnectionIteration};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

#[derive(Default)]
pub struct Inliner;

impl Named for Inliner {
    fn name() -> &'static str {
        "hole-inliner"
    }

    fn description() -> &'static str {
        "inlines holes"
    }
}

impl Visitor for Inliner {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        let st = &mut comp.structure;

        println!("hi");

        // This pass doesn't modify any control.
        Ok(Action::Stop)
    }
}
