use crate::environment::InterpreterState;
use calyx::ir::{Assignment, Port};

pub struct GroupInterpeter {}

impl GroupInterpeter {
    fn new<'a, I>(env: InterpreterState, done_signal: &Port, assigns: I) -> Self
    where
        I: Iterator<Item = &'a Assignment>,
    {
        todo!()
    }
    fn step_cycle(&mut self) {
        todo!();
    }
    fn step_convergence(&mut self) {
        todo!();
    }
    fn step_cycle_convergence(&mut self) {
        todo!();
    }

    fn run_group(&mut self) {}

    pub fn is_done(&self) -> bool {
        todo!()
    }

    fn deconstruct(self) -> InterpreterState {
        todo!()
    }
}
