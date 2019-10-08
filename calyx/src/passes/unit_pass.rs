use crate::ast::{Ifen, Namespace};
use crate::passes::visitor::{Visitable, Visitor};

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

pub fn do_nothing(n: &mut Namespace) -> Nothing {
    let mut nothing = Nothing {};
    n.visit(&mut nothing)
        .unwrap_or_else(|x| panic!("Nothing pass failed: {:?}", x));
    nothing
}
