use crate::context::Context;
use crate::lang::ast;
use crate::passes::visitor::{Action, VisResult, Visitor};

pub struct Test {}

impl Default for Test {
    fn default() -> Test {
        Test {}
    }
}

impl Visitor for Test {
    fn name(&self) -> String {
        String::from("Test pass for sanity checking")
    }

    fn start_enable(
        &mut self,
        en: &mut ast::Enable,
        _changes: &Context,
    ) -> VisResult {
        println!("found an enable! {:?}", en);
        Ok(Action::Continue)
    }
}
