use crate::passes::visitor::Visitor;

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
