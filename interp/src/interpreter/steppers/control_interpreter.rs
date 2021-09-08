use super::super::utils::get_done_port;
use super::{AssignmentInterpreter, AssignmentInterpreterMarker};
use crate::interpreter::interpret_group::finish_interpretation;
use crate::utils::AsRaw;
use crate::{
    environment::{
        CompositeView, InterpreterState, MutCompositeView, MutStateView,
        StateView,
    },
    errors::InterpreterResult,
    interpreter::utils::{is_signal_high, ConstPort, ReferenceHolder},
    values::Value,
};
use calyx::ir::{self, Assignment, Component, Control};
use itertools::{peek_nth, Itertools, PeekNth};
use std::cell::Ref;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::rc::Rc;

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

    fn converge(&mut self) -> InterpreterResult<()>;

    fn run(&mut self) -> InterpreterResult<()> {
        while !self.is_done() {
            self.step()?;
        }
        Ok(())
    }

    fn deconstruct(self) -> InterpreterState<'outer>;

    fn is_done(&self) -> bool;

    fn get_env(&self) -> StateView<'_, 'outer>;

    fn currently_executing_group(&self) -> Vec<&ir::Id>;

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer>;

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn AssignmentInterpreterMarker>;
}

pub struct EmptyInterpreter<'outer> {
    pub(super) env: InterpreterState<'outer>,
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

    fn get_env(&self) -> StateView<'_, 'outer> {
        (&self.env).into()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        vec![]
    }

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn AssignmentInterpreterMarker> {
        None
    }

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        (&mut self.env).into()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        Ok(())
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
        let interp = AssignmentInterpreter::new_owned(env, done, assigns);
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

    fn get_env(&self) -> StateView<'_, 'outer> {
        (self.interp.get_env()).into()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        if let Some(name) = &self.group_name {
            vec![name]
        } else {
            vec![]
        }
    }

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn AssignmentInterpreterMarker> {
        Some(&mut self.interp)
    }

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        (self.interp.get_mut_env()).into()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        self.interp.step_convergence()
    }
}

pub struct SeqInterpreter<'a, 'outer> {
    sequence: PeekNth<std::slice::Iter<'a, Control>>,
    current_interpreter: Option<ControlInterpreter<'a, 'outer>>,
    continuous_assignments: &'a [Assignment],
    env: Option<InterpreterState<'outer>>,
    done_flag: bool,
    input_ports: Rc<HashSet<*const ir::Port>>,
}
impl<'a, 'outer> SeqInterpreter<'a, 'outer> {
    pub fn new(
        seq: &'a ir::Seq,
        env: InterpreterState<'outer>,
        continuous_assigns: &'a [Assignment],
        input_ports: Rc<HashSet<*const ir::Port>>,
    ) -> Self {
        Self {
            sequence: peek_nth(seq.stmts.iter()),
            current_interpreter: None,
            continuous_assignments: continuous_assigns,
            env: Some(env),
            done_flag: false,
            input_ports,
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
                Rc::clone(&self.input_ports),
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

    fn get_env(&self) -> StateView<'_, 'outer> {
        if let Some(cur) = &self.current_interpreter {
            cur.get_env()
        } else if let Some(env) = &self.env {
            env.into()
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

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn AssignmentInterpreterMarker> {
        self.current_interpreter
            .as_mut()
            .map(|x| x.get_current_interp())
            .flatten()
    }

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        if let Some(cur) = &mut self.current_interpreter {
            cur.get_mut_env()
        } else if let Some(env) = &mut self.env {
            env.into()
        } else {
            unreachable!("Invalid internal state for SeqInterpreter")
        }
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        if let Some(cur) = &mut self.current_interpreter {
            cur.converge()
        } else {
            Ok(())
        }
    }
}

pub struct ParInterpreter<'a, 'outer> {
    interpreters: Vec<ControlInterpreter<'a, 'outer>>,
    in_state: InterpreterState<'outer>,
    input_ports: Rc<HashSet<*const ir::Port>>,
}

impl<'a, 'outer> ParInterpreter<'a, 'outer> {
    pub fn new(
        par: &'a ir::Par,
        mut env: InterpreterState<'outer>,
        continuous_assigns: &'a [Assignment],
        input_ports: Rc<HashSet<*const ir::Port>>,
    ) -> Self {
        let interpreters = par
            .stmts
            .iter()
            .map(|x| {
                ControlInterpreter::new(
                    x,
                    env.fork(),
                    continuous_assigns,
                    Rc::clone(&input_ports),
                )
            })
            .collect();

        Self {
            interpreters,
            in_state: env,
            input_ports,
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

        self.in_state.merge_many(envs, &self.input_ports)
    }

    fn is_done(&self) -> bool {
        self.interpreters.iter().all(|x| x.is_done())
    }

    fn get_env(&self) -> StateView<'_, 'outer> {
        CompositeView::new(
            &self.in_state,
            self.interpreters.iter().map(|x| x.get_env()).collect(),
        )
        .into()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        self.interpreters
            .iter()
            .flat_map(|x| x.currently_executing_group())
            .collect()
    }

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn AssignmentInterpreterMarker> {
        None
    }

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        MutCompositeView::new(
            &mut self.in_state,
            self.interpreters
                .iter_mut()
                .map(|x| x.get_mut_env())
                .collect(),
        )
        .into()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        for res in self
            .interpreters
            .iter_mut()
            .map(ControlInterpreter::converge)
        {
            // return first error
            if let err @ Err(_) = res {
                return err;
            }
        }
        Ok(())
    }
}
pub struct IfInterpreter<'a, 'outer> {
    port: ConstPort,
    cond: Option<EnableInterpreter<'a, 'outer>>,
    tbranch: &'a Control,
    fbranch: &'a Control,
    branch_interp: Option<ControlInterpreter<'a, 'outer>>,
    continuous_assignments: &'a [Assignment],
    input_ports: Rc<HashSet<*const ir::Port>>,
}

impl<'a, 'outer> IfInterpreter<'a, 'outer> {
    pub fn new(
        ctrl_if: &'a ir::If,
        env: InterpreterState<'outer>,
        continuous_assigns: &'a [Assignment],
        input_ports: Rc<HashSet<*const ir::Port>>,
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
            input_ports,
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
                        Rc::clone(&self.input_ports),
                    );
                } else {
                    let env = i.deconstruct();
                    branch = ControlInterpreter::new(
                        self.fbranch,
                        env,
                        self.continuous_assignments,
                        Rc::clone(&self.input_ports),
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

    fn get_env(&self) -> StateView<'_, 'outer> {
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

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn AssignmentInterpreterMarker> {
        match (&mut self.cond, &mut self.branch_interp) {
            (None, Some(x)) => x.get_current_interp(),
            (Some(x), None) => x.get_current_interp(),
            _ => unreachable!("If interpreter in invalid state"),
        }
    }

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        if let Some(cond) = &mut self.cond {
            cond.get_mut_env()
        } else if let Some(branch) = &mut self.branch_interp {
            branch.get_mut_env()
        } else {
            unreachable!("Invalid internal state for IfInterpreter")
        }
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        match (&mut self.cond, &mut self.branch_interp) {
            (None, Some(i)) => i.converge(),
            (Some(i), None) => i.converge(),
            _ => unreachable!("if interpreter in invalid internal state"),
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
    input_ports: Rc<HashSet<*const ir::Port>>,
}

impl<'a, 'outer> WhileInterpreter<'a, 'outer> {
    pub fn new(
        ctrl_while: &'a ir::While,
        env: InterpreterState<'outer>,
        continuous_assignments: &'a [Assignment],
        input_ports: Rc<HashSet<*const ir::Port>>,
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
            input_ports,
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
                        Rc::clone(&self.input_ports),
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

    fn get_env(&self) -> StateView<'_, 'outer> {
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
    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn AssignmentInterpreterMarker> {
        match (&mut self.cond_interp, &mut self.body_interp) {
            (None, Some(x)) => x.get_current_interp(),
            (Some(x), None) => x.get_current_interp(),
            _ => unreachable!("If interpreter in invalid state"),
        }
    }

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        if let Some(cond) = &mut self.cond_interp {
            cond.get_mut_env()
        } else if let Some(body) = &mut self.body_interp {
            body.get_mut_env()
        } else {
            unreachable!("Invalid internal state for WhileInterpreter")
        }
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        todo!()
    }
}
pub struct InvokeInterpreter<'a, 'outer> {
    invoke: &'a ir::Invoke,
    env: InterpreterState<'outer>,
}

impl<'a, 'outer> InvokeInterpreter<'a, 'outer> {
    pub fn new(_invoke: &ir::Invoke, _env: InterpreterState<'outer>) -> Self {
        todo!()
    }
}

impl<'a, 'outer> Interpreter<'outer> for InvokeInterpreter<'a, 'outer> {
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

    fn get_env(&self) -> StateView<'_, 'outer> {
        todo!()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        todo!()
    }

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn AssignmentInterpreterMarker> {
        todo!()
    }

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        todo!()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
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
    Invoke(Box<InvokeInterpreter<'a, 'outer>>),
}

impl<'a, 'outer> ControlInterpreter<'a, 'outer> {
    pub fn new(
        control: &'a Control,
        env: InterpreterState<'outer>,
        continuous_assignments: &'a [Assignment],
        input_ports: Rc<HashSet<*const ir::Port>>,
    ) -> Self {
        match control {
            Control::Seq(s) => Self::Seq(Box::new(SeqInterpreter::new(
                s,
                env,
                continuous_assignments,
                input_ports,
            ))),
            Control::Par(par) => Self::Par(Box::new(ParInterpreter::new(
                par,
                env,
                continuous_assignments,
                input_ports,
            ))),
            Control::If(i) => Self::If(Box::new(IfInterpreter::new(
                i,
                env,
                continuous_assignments,
                input_ports,
            ))),
            Control::While(w) => Self::While(Box::new(WhileInterpreter::new(
                w,
                env,
                continuous_assignments,
                input_ports,
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

    fn get_env(&self) -> StateView<'_, 'outer> {
        control_match!(self, i, i.get_env())
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        control_match!(self, i, i.currently_executing_group())
    }

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn AssignmentInterpreterMarker> {
        control_match!(self, i, i.get_current_interp())
    }

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        control_match!(self, i, i.get_mut_env())
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        control_match!(self, i, i.converge())
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
        env: InterpreterState<'outer>,
    ) -> Self {
        let comp_sig = comp.signature.borrow();
        let done_port = comp_sig.get_with_attr("done");
        let go_port = comp_sig.get_with_attr("go");
        let continuous_assignments = &comp.continuous_assignments;

        let interp = AssignmentInterpreter::new(
            env,
            Rc::clone(&done_port),
            (std::iter::empty(), continuous_assignments.iter()),
        );

        Self {
            interp,
            continuous: continuous_assignments,
            done_port: done_port.as_raw(),
            go_port: go_port.as_raw(),
        }
    }
}

impl<'a, 'outer> Interpreter<'outer> for StructuralInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        self.interp.step()
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
        let final_env = self.interp.deconstruct();
        finish_interpretation(final_env, self.done_port, self.continuous.iter())
            .unwrap()
    }

    fn run(&mut self) -> InterpreterResult<()> {
        self.interp.run()
    }

    fn is_done(&self) -> bool {
        self.interp.is_deconstructable()
    }

    fn get_env(&self) -> StateView<'_, 'outer> {
        self.interp.get_env().into()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        vec![]
    }

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn AssignmentInterpreterMarker> {
        Some(&mut self.interp)
    }

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        self.interp.get_mut_env().into()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        self.interp.step_convergence()
    }
}
