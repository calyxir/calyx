use super::super::utils::get_done_port;
use super::AssignmentInterpreter;
use crate::interpreter::interpret_group::finish_interpretation;
use crate::utils::AsRaw;
use crate::{
    environment::InterpreterState,
    errors::InterpreterResult,
    interpreter::utils::{is_signal_high, ConstPort, ReferenceHolder},
    values::Value,
};
use calyx::ir::{self, Assignment, Component, Control};
use itertools::{peek_nth, Itertools, PeekNth};
use std::cell::Ref;
use std::marker::PhantomData;

// this almost certainly doesn't need to exist but it can't be a trait fn with a
// default impl because it consumes self
macro_rules! run_and_deconstruct {
    ($name:ident) => {{
        $name.run()?;
        $name.deconstruct()
    }};
}

pub trait Interpreter<'outer> {
    fn step(&mut self) -> InterpreterResult<()>;

    fn run(&mut self) -> InterpreterResult<()> {
        while !self.is_done() {
            self.step()?;
        }
        Ok(())
    }

    fn deconstruct(self) -> InterpreterState<'outer>;

    fn is_done(&self) -> bool;

    fn get_env(&self) -> Vec<&InterpreterState<'outer>>;

    fn currently_executing_group(&self) -> Vec<&ir::Id>;
}

pub struct EmptyInterpreter<'outer> {
    env: InterpreterState<'outer>,
}

impl<'outer> EmptyInterpreter<'outer> {
    pub fn new(env: InterpreterState<'outer>) -> Self {
        Self { env }
    }
}

impl<'outer> Interpreter<'outer> for EmptyInterpreter<'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        Ok(())
    }

    fn run(&mut self) -> InterpreterResult<()> {
        Ok(())
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
        self.env
    }

    fn is_done(&self) -> bool {
        true
    }

    fn get_env(&self) -> Vec<&InterpreterState<'outer>> {
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

pub struct EnableInterpreter<'a, 'outer> {
    enable: EnableHolder<'a>,
    group_name: Option<ir::Id>,
    interp: AssignmentInterpreter<'a, 'outer>,
}

impl<'a, 'outer> EnableInterpreter<'a, 'outer> {
    pub fn new<E>(
        enable: E,
        group_name: Option<ir::Id>,
        env: InterpreterState<'outer>,
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

impl<'a, 'outer> EnableInterpreter<'a, 'outer> {
    fn reset(self) -> InterpreterState<'outer> {
        self.interp.reset(self.enable.assignments.iter())
    }
    fn get<P: AsRaw<ir::Port>>(&self, port: P) -> &Value {
        self.interp.get(port)
    }
}

impl<'a, 'outer> Interpreter<'outer> for EnableInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        self.interp.step()
    }

    fn run(&mut self) -> InterpreterResult<()> {
        self.interp.run()
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
        self.reset()
    }

    fn is_done(&self) -> bool {
        self.interp.is_deconstructable()
    }

    fn get_env(&self) -> Vec<&InterpreterState<'outer>> {
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

pub struct SeqInterpreter<'a, 'outer> {
    sequence: PeekNth<std::slice::Iter<'a, Control>>,
    current_interpreter: Option<ControlInterpreter<'a, 'outer>>,
    continuous_assignments: &'a [Assignment],
    env: Option<InterpreterState<'outer>>,
    done_flag: bool,
}
impl<'a, 'outer> SeqInterpreter<'a, 'outer> {
    pub fn new(
        seq: &'a ir::Seq,
        env: InterpreterState<'outer>,
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

impl<'a, 'outer> Interpreter<'outer> for SeqInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
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
                self.current_interpreter.as_mut().unwrap().step()?
            }
        } else if self.sequence.peek().is_none()
        // there is nothing left to do
        {
            self.done_flag = true
        }

        Ok(())
    }

    fn is_done(&self) -> bool {
        // we don't use peek here because that requires mutable access to the
        // iterator
        self.current_interpreter.is_none() && self.done_flag
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
        self.env.unwrap()
    }

    fn get_env(&self) -> Vec<&InterpreterState<'outer>> {
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

pub struct ParInterpreter<'a, 'outer> {
    interpreters: Vec<ControlInterpreter<'a, 'outer>>,
    in_state: InterpreterState<'outer>,
}

impl<'a, 'outer> ParInterpreter<'a, 'outer> {
    pub fn new(
        par: &'a ir::Par,
        mut env: InterpreterState<'outer>,
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

impl<'a, 'outer> Interpreter<'outer> for ParInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        for i in &mut self.interpreters {
            i.step()?;
        }
        Ok(())
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
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

    fn get_env(&self) -> Vec<&InterpreterState<'outer>> {
        self.interpreters.iter().flat_map(|x| x.get_env()).collect()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        self.interpreters
            .iter()
            .flat_map(|x| x.currently_executing_group())
            .collect()
    }
}
pub struct IfInterpreter<'a, 'outer> {
    port: ConstPort,
    cond: Option<EnableInterpreter<'a, 'outer>>,
    tbranch: &'a Control,
    fbranch: &'a Control,
    branch_interp: Option<ControlInterpreter<'a, 'outer>>,
    continuous_assignments: &'a [Assignment],
}

impl<'a, 'outer> IfInterpreter<'a, 'outer> {
    pub fn new(
        ctrl_if: &'a ir::If,
        env: InterpreterState<'outer>,
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

impl<'a, 'outer> Interpreter<'outer> for IfInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
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

                self.branch_interp = Some(branch);
                Ok(())
            } else {
                i.step()
            }
        } else {
            self.branch_interp.as_mut().unwrap().step()
        }
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
        self.branch_interp.unwrap().deconstruct()
    }

    fn is_done(&self) -> bool {
        self.cond.is_none()
            && self.branch_interp.is_some()
            && self.branch_interp.as_ref().unwrap().is_done()
    }

    fn get_env(&self) -> Vec<&InterpreterState<'outer>> {
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
pub struct WhileInterpreter<'a, 'outer> {
    port: ConstPort,
    cond: Ref<'a, ir::Group>,
    body: &'a Control,
    continuous_assignments: &'a [ir::Assignment],
    cond_interp: Option<EnableInterpreter<'a, 'outer>>,
    body_interp: Option<ControlInterpreter<'a, 'outer>>,
}

impl<'a, 'outer> WhileInterpreter<'a, 'outer> {
    pub fn new(
        ctrl_while: &'a ir::While,
        env: InterpreterState<'outer>,
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

impl<'a, 'outer> Interpreter<'outer> for WhileInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
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
                ci.step()?
            }
        } else if let Some(bi) = &mut self.body_interp {
            if !bi.is_done() {
                bi.step()?
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
        Ok(())
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
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

    fn get_env(&self) -> Vec<&InterpreterState<'outer>> {
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
pub struct InvokeInterpreter<'outer> {
    phantom: PhantomData<InterpreterState<'outer>>, // placeholder to force lifetime annotations
}

impl<'outer> InvokeInterpreter<'outer> {
    pub fn new(_invoke: &ir::Invoke, _env: InterpreterState<'outer>) -> Self {
        todo!()
    }
}

impl<'outer> Interpreter<'outer> for InvokeInterpreter<'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        todo!()
    }

    fn run(&mut self) -> InterpreterResult<()> {
        todo!()
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
        todo!()
    }

    fn is_done(&self) -> bool {
        todo!()
    }

    fn get_env(&self) -> Vec<&InterpreterState<'outer>> {
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

pub enum ControlInterpreter<'a, 'outer> {
    Empty(Box<EmptyInterpreter<'outer>>),
    Enable(Box<EnableInterpreter<'a, 'outer>>),
    Seq(Box<SeqInterpreter<'a, 'outer>>),
    Par(Box<ParInterpreter<'a, 'outer>>),
    If(Box<IfInterpreter<'a, 'outer>>),
    While(Box<WhileInterpreter<'a, 'outer>>),
    Invoke(Box<InvokeInterpreter<'outer>>),
}

impl<'a, 'outer> ControlInterpreter<'a, 'outer> {
    pub fn new(
        control: &'a Control,
        env: InterpreterState<'outer>,
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

impl<'a, 'outer> Interpreter<'outer> for ControlInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        control_match!(self, i, i.step())
    }

    fn run(&mut self) -> InterpreterResult<()> {
        control_match!(self, i, i.run())
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
        control_match!(self, i, i.deconstruct())
    }

    fn is_done(&self) -> bool {
        control_match!(self, i, i.is_done())
    }

    fn get_env(&self) -> Vec<&InterpreterState<'outer>> {
        control_match!(self, i, i.get_env())
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        control_match!(self, i, i.currently_executing_group())
    }
}

pub struct StructuralInterpreter<'a, 'outer> {
    interp: AssignmentInterpreter<'a, 'outer>,
    continuous: &'a [Assignment],
    done_port: ConstPort,
    go_port: ConstPort,
}

impl<'a, 'outer> StructuralInterpreter<'a, 'outer> {
    pub fn from_component(
        comp: &'a Component,
        mut env: InterpreterState<'outer>,
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

impl<'a, 'outer> Interpreter<'outer> for StructuralInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        self.interp.step()
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
        let mut final_env = self.interp.deconstruct();
        final_env.insert(self.go_port, Value::bit_low());
        finish_interpretation(final_env, self.done_port, self.continuous.iter())
            .unwrap()
    }

    fn run(&mut self) -> InterpreterResult<()> {
        self.interp.run()
    }

    fn is_done(&self) -> bool {
        self.interp.is_deconstructable()
    }

    fn get_env(&self) -> Vec<&InterpreterState<'outer>> {
        vec![self.interp.get_env()]
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        vec![]
    }
}
