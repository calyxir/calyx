use crate::backend::fsm::machine::{State, FSM};
use crate::lang::ast::{Component, Enable, Seq};
use crate::passes::visitor::Visitor;
pub struct FsmList {
    list: Vec<FSM>,
}

impl Visitor<()> for FsmList {
    fn new() -> FsmList {
        FsmList { list: vec![] }
    }

    fn name(&self) -> String {
        "FSM".to_string()
    }
    fn start_enable(&mut self, en: &mut Enable) -> Result<(), ()> {
        let outputs = en
            .clone()
            .comps
            .into_iter()
            .map(|x| ((x, "val".to_string()), 1))
            .collect();
        let en_state = State {
            outputs,
            transitions: vec![],
            default: None,
        };
        let fsm = FSM::new(en_state);

        self.list.push(fsm);
        Ok(())
    }

    fn finish_seq(
        &mut self,
        _seq: &mut Seq,
        _: Result<(), ()>,
    ) -> Result<(), ()> {
        //println!("{:#?}", seq);
        for i in 0..(self.list.len() - 1) {
            if self.list[i + 1].states.len() > 0 {
                let next = self.list[i + 1].states[0].clone();

                let current = &mut self.list[i];
                let last_idx = current.states.len() - 1;
                let os = &current.states[last_idx].outputs;
                let condition = os
                    .iter()
                    .map(|((id, _), _)| ((id.clone(), "rdy".to_string()), 1))
                    .collect();

                current.states[last_idx]
                    .transitions
                    .push((condition, next.clone()));
            }
        }
        Ok(())
    }

    fn finish_component(
        &mut self,
        _c: &mut Component,
        _res: Result<(), ()>,
    ) -> Result<(), ()> {
        println!("{:#?}", self.list);
        Ok(())
    }
}
