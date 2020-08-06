use crate::lang::component::Component;
use crate::lang::{ast, context::Context, structure};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

#[derive(Default)]
pub struct UnitPass;

impl Named for UnitPass {
    fn name() -> &'static str {
        "unit-pass"
    }

    fn description() -> &'static str {
        "template code to copy when writing a new pass"
    }
}

impl Visitor for UnitPass {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        Ok(Action::Continue)
    }
}
