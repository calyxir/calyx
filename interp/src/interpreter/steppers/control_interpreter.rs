use super::super::simulation_utils::get_done_port;
use super::AssignmentInterpreter;
use crate::{
    environment::InterpreterState,
    interpreter::simulation_utils::{is_signal_high, ConstPort},
    values::Value,
};
use calyx::ir::{self, Assignment, Control, Group};
use itertools::{peek_nth, Itertools, PeekNth};
use std::cell::Ref;

macro_rules! run_and_deconstruct {
    ($name:ident) => {{
        $name.run();
        $name.deconstruct()
    }};
}

pub trait Interpreter {
    fn step(&mut self);

    fn run(&mut self) {
        while !self.is_done() {
            self.step()
        }
    }

    fn deconstruct(self) -> InterpreterState;

    fn is_done(&self) -> bool;
}

pub struct EmptyInterpreter<'a> {
    _empty: &'a ir::Empty,
    env: InterpreterState,
    _continuous: &'a [Assignment],
}

impl<'a> EmptyInterpreter<'a> {
    pub fn new(
        empty: &'a ir::Empty,
        env: InterpreterState,
        continuous_assignments: &'a [Assignment],
    ) -> Self {
        Self {
            _empty: empty,
            env,
            _continuous: continuous_assignments,
        }
    }
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

pub enum EnableBox<'a> {
    Enable(Ref<'a, ir::Group>),
    Group(&'a ir::Group),
}

impl<'a> EnableBox<'a> {
    fn get_grp(&self) -> &ir::Group {
        match self {
            EnableBox::Enable(g) => g,
            EnableBox::Group(g) => g,
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &Assignment> + '_> {
        match self {
            EnableBox::Enable(g) => Box::new(g.assignments.iter()),
            EnableBox::Group(g) => Box::new(g.assignments.iter()),
        }
    }
}

impl<'a> From<&'a ir::Enable> for EnableBox<'a> {
    fn from(en: &'a ir::Enable) -> Self {
        Self::Enable(en.group.borrow())
    }
}

impl<'a> From<&'a ir::Group> for EnableBox<'a> {
    fn from(g: &'a ir::Group) -> Self {
        Self::Group(g)
    }
}

impl<'a> From<Ref<'a, ir::Group>> for EnableBox<'a> {
    fn from(g: Ref<'a, ir::Group>) -> Self {
        Self::Enable(g)
    }
}

pub struct EnableInterpreter<'a> {
    enable: EnableBox<'a>,
    interp: AssignmentInterpreter<'a>,
}

impl<'a> EnableInterpreter<'a> {
    pub fn new<E>(
        enable: E,
        env: InterpreterState,
        continuous: &'a [Assignment],
    ) -> Self
    where
        E: Into<EnableBox<'a>>,
    {
        let enable: EnableBox = enable.into();
        let assigns = enable.iter().chain(continuous.iter()).cloned();
        let done = get_done_port(enable.get_grp());
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
        self.interp.reset(self.enable.iter())
    }
    fn get(&self, port: ConstPort) -> &Value {
        self.interp.get_val(port)
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
                self.sequence.next().unwrap(),
                self.env.take().unwrap(),
                self.continuous_assignments,
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

    fn is_done(&self) -> bool {
        self.current_interpreter.is_none() && self.done_flag
    }

    fn deconstruct(self) -> InterpreterState {
        self.env.unwrap()
    }
}

pub struct ParInterpreter<'a> {
    interpreters: Vec<ControlInterpreter<'a>>,
}

impl<'a> ParInterpreter<'a> {
    pub fn new(
        par: &'a ir::Par,
        mut env: InterpreterState,
        continuous_assigns: &'a [Assignment],
    ) -> Self {
        let interpreters = par
            .stmts
            .iter()
            .map(|x| ControlInterpreter::new(x, env.fork(), continuous_assigns))
            .collect();

        Self { interpreters }
    }
}

impl<'a> Interpreter for ParInterpreter<'a> {
    fn step(&mut self) {
        for i in &mut self.interpreters {
            i.step()
        }
    }

    fn deconstruct(self) -> InterpreterState {
        // need to incorporate stk_env stuff
        todo!()
    }

    fn is_done(&self) -> bool {
        self.interpreters.iter().all(|x| x.is_done())
    }
}
pub struct IfInterpreter<'a> {
    port: ConstPort,
    cond: Option<EnableInterpreter<'a>>,
    tbranch: &'a Control,
    fbranch: &'a Control,
    branch_interp: Option<ControlInterpreter<'a>>,
    continuous_assignments: &'a [Assignment],
}

impl<'a> IfInterpreter<'a> {
    pub fn new(
        ctrl_if: &'a ir::If,
        env: InterpreterState,
        continuous_assigns: &'a [Assignment],
    ) -> Self {
        let port: ConstPort = ctrl_if.port.as_ptr();
        let cond = EnableInterpreter::new(
            ctrl_if.cond.borrow(),
            env,
            continuous_assigns,
        );
        Self {
            port,
            cond: Some(cond),
            tbranch: &ctrl_if.tbranch,
            fbranch: &ctrl_if.fbranch,
            branch_interp: None,
            continuous_assignments: continuous_assigns,
        }
    }
}

impl<'a> Interpreter for IfInterpreter<'a> {
    fn step(&mut self) {
        if let Some(i) = &mut self.cond {
            if i.is_done() {
                let i = self.cond.take().unwrap();
                let branch;
                if is_signal_high(i.get(self.port).into()) {
                    let env = i.deconstruct();
                    branch = ControlInterpreter::new(
                        self.tbranch,
                        env,
                        self.continuous_assignments,
                    );
                } else {
                    let env = i.deconstruct();
                    branch = ControlInterpreter::new(
                        self.fbranch,
                        env,
                        self.continuous_assignments,
                    );
                }

                self.branch_interp = Some(branch)
            } else {
                i.step()
            }
        } else {
            self.branch_interp.as_mut().unwrap().step()
        }
    }

    fn deconstruct(self) -> InterpreterState {
        self.branch_interp.unwrap().deconstruct()
    }

    fn is_done(&self) -> bool {
        self.cond.is_none()
            && self.branch_interp.is_some()
            && self.branch_interp.as_ref().unwrap().is_done()
    }
}
pub struct WhileInterpreter<'a> {
    port: ConstPort,
    cond: Ref<'a, ir::Group>,
    body: &'a Control,
    continuous_assignments: &'a [ir::Assignment],
    cond_interp: Option<EnableInterpreter<'a>>,
    body_interp: Option<ControlInterpreter<'a>>,
}

impl<'a> WhileInterpreter<'a> {
    pub fn new(
        ctrl_while: &'a ir::While,
        env: InterpreterState,
        continuous_assignments: &'a [Assignment],
    ) -> Self {
        let port: ConstPort = ctrl_while.port.as_ptr();
        let cond = ctrl_while.cond.borrow();
        let cond_interp = EnableInterpreter::new(
            Ref::clone(&cond),
            env,
            continuous_assignments,
        );
        Self {
            port,
            cond,
            body: &ctrl_while.body,
            continuous_assignments,
            cond_interp: Some(cond_interp),
            body_interp: None,
        }
    }
}

impl<'a> Interpreter for WhileInterpreter<'a> {
    fn step(&mut self) {
        if let Some(ci) = &mut self.cond_interp {
            if ci.is_done() {
                let ci = self.cond_interp.take().unwrap();
                if is_signal_high(ci.get(self.port).into()) {
                    let body_interp = ControlInterpreter::new(
                        self.body,
                        ci.deconstruct(),
                        self.continuous_assignments,
                    );
                    self.body_interp = Some(body_interp)
                } else {
                    self.cond_interp = Some(ci)
                }
            } else {
                ci.step()
            }
        } else if let Some(bi) = &mut self.body_interp {
            if !bi.is_done() {
                bi.step()
            } else {
                let bi = self.body_interp.take().unwrap();
                let cond_interp = EnableInterpreter::new(
                    Ref::clone(&self.cond),
                    bi.deconstruct(),
                    self.continuous_assignments,
                );
                self.cond_interp = Some(cond_interp)
            }
        } else {
            panic!("While Interpreter is in an invalid state")
        }
    }

    fn deconstruct(self) -> InterpreterState {
        self.cond_interp.unwrap().deconstruct()
    }

    fn is_done(&self) -> bool {
        self.body_interp.is_none()
            && self.cond_interp.is_some()
            && !is_signal_high(
                self.cond_interp.as_ref().unwrap().get(self.port).into(),
            )
    }
}
pub struct InvokeInterpreter {}

impl InvokeInterpreter {
    pub fn new(invoke: &ir::Invoke, env: InterpreterState) -> Self {
        todo!()
    }
}

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
    Empty(Box<EmptyInterpreter<'a>>),
    Enable(Box<EnableInterpreter<'a>>),
    Seq(Box<SeqInterpreter<'a>>),
    Par(Box<ParInterpreter<'a>>),
    If(Box<IfInterpreter<'a>>),
    While(Box<WhileInterpreter<'a>>),
    Invoke(Box<InvokeInterpreter>),
}

impl<'a> ControlInterpreter<'a> {
    pub fn new(
        control: &'a Control,
        env: InterpreterState,
        continuous_assignments: &'a [Assignment],
    ) -> Self {
        match control {
            Control::Seq(s) => Self::Seq(Box::new(SeqInterpreter::new(
                s,
                env,
                continuous_assignments,
            ))),
            Control::Par(par) => Self::Par(Box::new(ParInterpreter::new(
                par,
                env,
                continuous_assignments,
            ))),
            Control::If(i) => Self::If(Box::new(IfInterpreter::new(
                i,
                env,
                continuous_assignments,
            ))),
            Control::While(w) => Self::While(Box::new(WhileInterpreter::new(
                w,
                env,
                continuous_assignments,
            ))),
            Control::Invoke(i) => {
                Self::Invoke(Box::new(InvokeInterpreter::new(i, env)))
            }
            Control::Enable(e) => Self::Enable(Box::new(
                EnableInterpreter::new(e, env, continuous_assignments),
            )),
            Control::Empty(e) => Self::Empty(Box::new(EmptyInterpreter::new(
                e,
                env,
                continuous_assignments,
            ))),
        }
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
