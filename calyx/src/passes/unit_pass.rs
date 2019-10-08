use crate::ast::Ifen;
use crate::passes::visitor::Visitor;

pub struct Nothing {}

impl Visitor<()> for Nothing {
    fn new() -> Nothing {
        Nothing {}
    }

    fn name(&self) -> String {
        "Nothing".to_string()
    }

    fn start_ifen(&mut self, con: &mut Ifen) -> Result<(), ()> {
        println!("{:#?}", con.cond);
        Ok(())
    }
}
