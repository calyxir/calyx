use crate::ast;
use crate::ast::Namespace;
use crate::pass::{Visitable, Visitor};

pub struct Nothing {}

impl Visitor<()> for Nothing {
    fn start_ifen(&mut self, con: &mut ast::Ifen) -> Result<(), ()> {
        println!("{:#?}", con.cond);
        Ok(())
    }
}

pub fn do_nothing(n: &mut Namespace) -> Nothing {
    let mut nothing = Nothing {};

    n.visit(&mut nothing)
        .unwrap_or_else(|x| panic!("Nothing pass failed: {:?}", x));
    nothing
}
