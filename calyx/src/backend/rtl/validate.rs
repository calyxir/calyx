use crate::context::Context;
use crate::lang::ast;
use crate::lang::component::Component;
use crate::passes::visitor::{Action, VisResult, Visitor};

pub struct Validate {}

impl Default for Validate {
    fn default() -> Self {
        Validate {}
    }
}

impl Visitor for Validate {
    fn name(&self) -> String {
        "Rtl Validator".to_string()
    }
}
