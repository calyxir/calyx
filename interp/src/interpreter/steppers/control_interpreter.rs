use super::super::utils::get_done_port;
use super::AssignmentInterpreter;
use crate::interpreter::interpret_group::finish_interpretation;
use crate::utils::AsRaw;
use crate::{
    environment::InterpreterState,
    interpreter::utils::{is_signal_high, ConstPort, ReferenceHolder},
    values::Value,
};
use calyx::ir::{self, Assignment, Component, Control};
use itertools::{peek_nth, Itertools, PeekNth};
use std::cell::Ref;

// this almost certainly doesn't need to exist but it can't be a trait fn with a
// default impl because it consumes self
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

    fn get_env(&self) -> Vec<&InterpreterState>;

    fn currently_executing_group(&self) -> Vec<&ir::Id>;
}

pub struct EmptyInterpreter {
    env: InterpreterState,
}

impl EmptyInterpreter {
    pub fn new(env: InterpreterState) -> Self {
        Self { env }
    }
}

impl Interpreter for EmptyInterpreter {
    fn step(&mut self) {}

    fn run(&mut self) {}

    fn deconstruct(self) -> InterpreterState {
        self.env
    }

    fn is_done(&self) -> bool {
        true
    }

    fn get_env(&self) -> Vec<&InterpreterState> {
        vec![&self.env]
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        vec![]
    }
}

type EnableHolder<'a> = ReferenceHolder<'a, ir::Group>;

impl<'a> From<&'a ir::Enable> for ReferenceHolder<'a, ir::Group> {
    fn from(e: &'a ir::Enable) -> Self {
        e.group.borrow().into()
    }
}

pub struct EnableInterpreter<'a> {
    enable: EnableHolder<'a>,
    group_name: Option<ir::Id>,
    interp: AssignmentInterpreter<'a>,
}

impl<'a> EnableInterpreter<'a> {
    pub fn new<E>(
        enable: E,
        group_name: Option<ir::Id>,
        env: InterpreterState,
        continuous: &'a [Assignment],
    ) -> Self
    where
        E: Into<EnableHolder<'a>>,
    {
        let enable: EnableHolder = enable.into();
        let assigns = (
            enable.assignments.iter().cloned().collect_vec(),
            continuous.iter().cloned().collect_vec(),
        );
        let done = get_done_port(&enable);
        let interp = AssignmentInterpreter::new_owned(
            env,
            &done.borrow() as &ir::Port as *const ir::Port,
            assigns,
        );
        Self {
            enable,
            group_name,
            interp,
        }
    }
}

impl<'a> EnableInterpreter<'a> {
    fn reset(self) -> InterpreterState {
        self.interp.reset(self.enable.assignments.iter())
    }
    fn get<P: AsRaw<ir::Port>>(&self, port: P) -> &Value {
        self.interp.get(port)
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

    fn get_env(&self) -> Vec<&InterpreterState> {
        vec![self.interp.get_env()]
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        if let Some(name) = &self.group_name {
            vec![name]
        } else {
            vec![]
        }
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
        // we don't use peek here because that requires mutable access to the
        // iterator
        self.current_interpreter.is_none() && self.done_flag
    }

    fn deconstruct(self) -> InterpreterState {
        self.env.unwrap()
    }

    fn get_env(&self) -> Vec<&InterpreterState> {
        if let Some(cur) = &self.current_interpreter {
            cur.get_env()
        } else if let Some(env) = &self.env {
            vec![env]
        } else {
            unreachable!("Invalid internal state for SeqInterpreter")
        }
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        if let Some(grp) = &self.current_interpreter {
            grp.currently_executing_group()
        } else {
            vec![]
        }
    }
}

pub struct ParInterpreter<'a> {
    interpreters: Vec<ControlInterpreter<'a>>,
    in_state: InterpreterState,
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

        Self {
            interpreters,
            in_state: env,
        }
    }
}

impl<'a> Interpreter for ParInterpreter<'a> {
    fn step(&mut self) {
        for i in &mut self.interpreters {
            i.step()
        }
    }

    fn deconstruct(self) -> InterpreterState {
        assert!(self.interpreters.iter().all(|x| x.is_done()));
        let envs = self
            .interpreters
            .into_iter()
            .map(ControlInterpreter::deconstruct)
            .collect_vec();

        self.in_state.merge_many(envs)
    }

    fn is_done(&self) -> bool {
        self.interpreters.iter().all(|x| x.is_done())
    }

    fn get_env(&self) -> Vec<&InterpreterState> {
        self.interpreters.iter().flat_map(|x| x.get_env()).collect()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        self.interpreters
            .iter()
            .flat_map(|x| x.currently_executing_group())
            .collect()
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
            Some(ctrl_if.cond.borrow().name().clone()),
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
                #[allow(clippy::branches_sharing_code)]
                if is_signal_high(i.get(self.port)) {
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

    fn get_env(&self) -> Vec<&InterpreterState> {
        if let Some(cond) = &self.cond {
            cond.get_env()
        } else if let Some(branch) = &self.branch_interp {
            branch.get_env()
        } else {
            unreachable!("Invalid internal state for IfInterpreter")
        }
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        if let Some(grp) = &self.cond {
            grp.currently_executing_group()
        } else if let Some(branch) = &self.branch_interp {
            branch.currently_executing_group()
        } else {
            vec![]
        }
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
            Some(cond.name().clone()),
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
                if is_signal_high(ci.get(self.port)) {
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
                    Some(self.cond.name().clone()),
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
            && self.cond_interp.as_ref().unwrap().is_done()
            && !is_signal_high(
                self.cond_interp.as_ref().unwrap().get(self.port),
            )
    }

    fn get_env(&self) -> Vec<&InterpreterState> {
        if let Some(cond) = &self.cond_interp {
            cond.get_env()
        } else if let Some(body) = &self.body_interp {
            body.get_env()
        } else {
            unreachable!("Invalid internal state for WhileInterpreter")
        }
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        if let Some(cond) = &self.cond_interp {
            cond.currently_executing_group()
        } else if let Some(body) = &self.body_interp {
            body.currently_executing_group()
        } else {
            vec![]
        }
    }
}
pub struct InvokeInterpreter {}

impl InvokeInterpreter {
    pub fn new(_invoke: &ir::Invoke, _env: InterpreterState) -> Self {
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

    fn get_env(&self) -> Vec<&InterpreterState> {
        todo!()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        todo!()
    }
}

// internal use macro that just captures the same name and expression for each
// arm of the control interpreter match. This is largely as a convenience
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
    Empty(Box<EmptyInterpreter>),
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
            Control::Enable(e) => {
                Self::Enable(Box::new(EnableInterpreter::new(
                    e,
                    Some(e.group.borrow().name().clone()),
                    env,
                    continuous_assignments,
                )))
            }
            Control::Empty(_) => {
                Self::Empty(Box::new(EmptyInterpreter::new(env)))
            }
        }
    }
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

    fn get_env(&self) -> Vec<&InterpreterState> {
        control_match!(self, i, i.get_env())
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        control_match!(self, i, i.currently_executing_group())
    }
}

pub struct StructuralInterpreter<'a> {
    interp: AssignmentInterpreter<'a>,
    continuous: &'a [Assignment],
    done_port: ConstPort,
    go_port: ConstPort,
}

impl<'a> StructuralInterpreter<'a> {
    pub fn from_component(
        comp: &'a Component,
        mut env: InterpreterState,
    ) -> Self {
        let comp_sig = comp.signature.borrow();
        let done_port: ConstPort = comp_sig.get("done").as_ptr();
        let go_port: ConstPort = comp_sig.get("go").as_ptr();
        let continuous_assignments = &comp.continuous_assignments;

        if !is_signal_high(env.get_from_port(done_port)) {
            env.insert(go_port, Value::bit_high());
        }

        let interp = AssignmentInterpreter::new(
            env,
            done_port,
            (std::iter::empty(), continuous_assignments.iter()),
        );

        Self {
            interp,
            continuous: continuous_assignments,
            done_port,
            go_port,
        }
    }
}

impl<'a> Interpreter for StructuralInterpreter<'a> {
    fn step(&mut self) {
        self.interp.step();
    }

    fn deconstruct(self) -> InterpreterState {
        let mut final_env = self.interp.deconstruct();
        final_env.insert(self.go_port, Value::bit_low());
        finish_interpretation(final_env, self.done_port, self.continuous.iter())
            .unwrap()
    }

    fn run(&mut self) {
        self.interp.run();
    }

    fn is_done(&self) -> bool {
        self.interp.is_deconstructable()
    }

    fn get_env(&self) -> Vec<&InterpreterState> {
        vec![self.interp.get_env()]
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        vec![]
    }
}
