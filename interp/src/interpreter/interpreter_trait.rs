use std::collections::HashSet;

use crate::{
    debugger::name_tree::ActiveTreeNode,
    environment::{InterpreterState, MutStateView, StateView},
    errors::InterpreterResult,
    structures::names::GroupQIN,
};

pub trait Interpreter {
    fn step(&mut self) -> InterpreterResult<()>;

    fn converge(&mut self) -> InterpreterResult<()>;

    fn run(&mut self) -> InterpreterResult<()> {
        while !self.is_done() {
            self.step()?;
        }
        Ok(())
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState>;

    fn is_done(&self) -> bool;

    fn get_env(&self) -> StateView<'_>;

    fn currently_executing_group(&self) -> HashSet<GroupQIN>;

    fn get_mut_env(&mut self) -> MutStateView<'_>;

    fn get_active_tree(&self) -> Vec<ActiveTreeNode>;

    fn run_and_deconstruct(mut self) -> InterpreterResult<InterpreterState>
    where
        Self: Sized,
    {
        self.run()?;
        self.deconstruct()
    }
}
