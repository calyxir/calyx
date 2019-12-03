use crate::backend::fsm::machine::{State, FSM};
use crate::lang::ast::Seq;
use crate::passes::visitor::Visitor;

impl Visitor<()> for FSM {
    fn new() -> FSM {
        FSM {
            inputs: vec![],
            outputs: vec![],
            states: vec![],
            start: State::empty(),
        }
    }

    fn name(&self) -> String {
        "FSM".to_string()
    }

    fn start_seq(&mut self, seq: &mut Seq) -> Result<(), ()> {
        println!("{:#?}", seq);
        Ok(())
    }
}
