use super::super::simulation_utils::get_done_port;
use super::AssignmentInterpreter;
use crate::environment::InterpreterState;
use calyx::ir::{self, Assignment, Control, Group};
use itertools::{peek_nth, Itertools, PeekNth};

macro_rules! run_and_deconstruct {
    ($name:ident) => {{
        $name.run();
        $name.deconstruct()
    }};
}

pub trait Interpreter {
    fn step(&mut self);

    fn run(&mut self);

    fn deconstruct(self) -> InterpreterState;

    fn is_done(&self) -> bool;
}

pub struct EmptyInterpreter<'a> {
    env: InterpreterState,
    _continuous: &'a [Assignment],
    _empty: &'a ir::Empty,
}

impl<'a> Interpreter for EmptyInterpreter<'a> {
    fn step(&mut self) {}

    fn run(&mut self) {}

    fn deconstruct(self) -> InterpreterState {
        self.env
    }

    fn is_done(&self) -> bool {
        true
    }
}

pub struct EnableInterpreter<'a> {
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
        Self { enable, interp }
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

    fn deconstruct(self) -> InterpreterState {
        self.reset()
    }

    fn is_done(&self) -> bool {
        self.interp.is_deconstructable()
    }
}

pub struct SeqInterpreter<'a> {
    sequence: PeekNth<std::slice::Iter<'a, Control>>,
    current_interpreter: Option<ControlInterpreter<'a>>,
    continuous_assignments: &'a [Assignment],
    env: Option<InterpreterState>,
    done_flag: bool,
}
impl<'a> SeqInterpreter<'a> {
    pub fn new(
        seq: &'a ir::Seq,
        env: InterpreterState,
        continuous_assigns: &'a [Assignment],
    ) -> Self {
        Self {
            sequence: peek_nth(seq.stmts.iter()),
            current_interpreter: None,
            continuous_assignments: continuous_assigns,
            env: Some(env),
            done_flag: false,
        }
    }
}

impl<'a> Interpreter for SeqInterpreter<'a> {
    fn step(&mut self) {
        if self.current_interpreter.is_none() && self.sequence.peek().is_some()
        // There is more to execute, make new interpreter
        {
            self.current_interpreter = ControlInterpreter::new(
                self.continuous_assignments,
                self.sequence.next().unwrap(),
                self.env.take().unwrap(),
            )
            .into()
        } else if self.current_interpreter.is_some()
        // current interpreter can be stepped/deconstructed
        {
            if self.current_interpreter.as_ref().unwrap().is_done() {
                let mut interp = self.current_interpreter.take().unwrap();
                let res = run_and_deconstruct!(interp);
                self.env = Some(res);
            } else {
                self.current_interpreter.as_mut().unwrap().step()
            }
        } else if self.sequence.peek().is_none()
        // there is nothing left to do
        {
            self.done_flag = true
        }
    }

    fn run(&mut self) {
        while !self.is_done() {
            self.step()
        }
    }

    fn is_done(&self) -> bool {
        self.current_interpreter.is_none() && self.done_flag
    }

    fn deconstruct(self) -> InterpreterState {
        self.env.unwrap()
    }
}

pub struct ParInterpreter {}

impl Interpreter for ParInterpreter {
    fn step(&mut self) {
        todo!()
    }

    fn run(&mut self) {
        todo!()
    }

    fn deconstruct(self) -> InterpreterState {
        todo!()
    }

    fn is_done(&self) -> bool {
        todo!()
    }
}
pub struct IfInterpreter {}

impl Interpreter for IfInterpreter {
    fn step(&mut self) {
        todo!()
    }

    fn run(&mut self) {
        todo!()
    }

    fn deconstruct(self) -> InterpreterState {
        todo!()
    }

    fn is_done(&self) -> bool {
        todo!()
    }
}
pub struct WhileInterpreter {}

impl Interpreter for WhileInterpreter {
    fn step(&mut self) {
        todo!()
    }

    fn run(&mut self) {
        todo!()
    }

    fn deconstruct(self) -> InterpreterState {
        todo!()
    }

    fn is_done(&self) -> bool {
        todo!()
    }
}
pub struct InvokeInterpreter {}

impl Interpreter for InvokeInterpreter {
    fn step(&mut self) {
        todo!()
    }

    fn run(&mut self) {
        todo!()
    }

    fn deconstruct(self) -> InterpreterState {
        todo!()
    }

    fn is_done(&self) -> bool {
        todo!()
    }
}

macro_rules! control_match {
    ($matched: ident, $name:ident, $exp:expr) => {{
        match $matched {
            ControlInterpreter::Empty($name) => $exp,
            ControlInterpreter::Enable($name) => $exp,
            ControlInterpreter::Seq($name) => $exp,
            ControlInterpreter::Par($name) => $exp,
            ControlInterpreter::If($name) => $exp,
            ControlInterpreter::While($name) => $exp,
            ControlInterpreter::Invoke($name) => $exp,
        }
    }};
}

pub enum ControlInterpreter<'a> {
    Empty(Box<EnableInterpreter<'a>>),
    Enable(Box<EnableInterpreter<'a>>),
    Seq(Box<SeqInterpreter<'a>>),
    Par(Box<ParInterpreter>),
    If(Box<IfInterpreter>),
    While(Box<WhileInterpreter>),
    Invoke(Box<InvokeInterpreter>),
}

impl<'a> ControlInterpreter<'a> {
    pub fn new(
        continuous_assignments: &[Assignment],
        control: &Control,
        env: InterpreterState,
    ) -> Self {
        todo!()
    }

    // fn as_dyn_mut(&mut self) -> &mut dyn Interpreter {
    //     control_match!(self, inner, &mut (**inner))
    // }
    // fn as_dyn(&self) -> &dyn Interpreter {
    //     control_match!(self, inner, &(**inner))
    // }
}

impl<'a> Interpreter for ControlInterpreter<'a> {
    fn step(&mut self) {
        control_match!(self, i, i.step())
    }

    fn run(&mut self) {
        control_match!(self, i, i.run())
    }

    fn deconstruct(self) -> InterpreterState {
        control_match!(self, i, i.deconstruct())
    }

    fn is_done(&self) -> bool {
        control_match!(self, i, i.is_done())
    }
}
