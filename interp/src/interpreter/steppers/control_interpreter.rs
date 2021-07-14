use super::super::simulation_utils::get_done_port;
use super::AssignmentInterpreter;
use crate::environment::InterpreterState;
use calyx::ir::{self, Assignment, Control, Group};

pub trait Interpreter {
    fn step(&mut self);

    fn run(&mut self);

    fn run_and_deconstruct(self) -> InterpreterState;
}

pub struct EmptyInterpreter<'a> {
    env: InterpreterState,
    _continuous: &'a [Assignment],
    _empty: &'a ir::Empty,
}

impl<'a> Interpreter for EmptyInterpreter<'a> {
    fn step(&mut self) {}

    fn run(&mut self) {}

    fn run_and_deconstruct(self) -> InterpreterState {
        self.env
    }
}

pub struct EnableInterpreter<'a> {
    continuous: &'a [Assignment],
    enable: &'a ir::Enable,
    interp: AssignmentInterpreter<'a>,
}

impl<'a> EnableInterpreter<'a> {
    pub fn new(
        env: InterpreterState,
        continuous: &'a [Assignment],
        enable: &'a ir::Enable,
    ) -> Self {
        let grp_ref = enable.group.borrow();
        let assigns =
            grp_ref.assignments.iter().chain(continuous.iter()).cloned();
        let done = get_done_port(&grp_ref);
        let interp = AssignmentInterpreter::new_owned(
            env,
            &done.borrow() as &ir::Port as *const ir::Port,
            assigns.collect(),
        );
        Self {
            continuous,
            enable,
            interp,
        }
    }
}

impl<'a> EnableInterpreter<'a> {
    fn reset(self) -> InterpreterState {
        self.interp
            .reset(self.enable.group.borrow().assignments.iter())
    }
}

impl<'a> Interpreter for EnableInterpreter<'a> {
    fn step(&mut self) {
        self.interp.step();
    }

    fn run(&mut self) {
        self.interp.run();
    }

    fn run_and_deconstruct(mut self) -> InterpreterState {
        self.interp.run();
        self.reset()
    }
}

pub struct SeqInterpreter {}
pub struct ParInterpreter {}
pub struct IfInterpreter {}
pub struct WhileInterpreter {}
pub struct InvokeInterpreter {}

pub enum ControlInterpreter<'a> {
    Empty(EnableInterpreter<'a>),
    Enable(EnableInterpreter<'a>),
    Seq(SeqInterpreter),
    Par(ParInterpreter),
    If(IfInterpreter),
    While(WhileInterpreter),
    Invoke(InvokeInterpreter),
}
