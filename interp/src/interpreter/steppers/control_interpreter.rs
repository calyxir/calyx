use super::super::utils::{get_done_port, get_go_port};
use super::AssignmentInterpreter;
use crate::errors::InterpreterError;
use crate::interpreter::interpret_group::finish_interpretation;
use crate::utils::AsRaw;
use crate::{
    environment::{
        CompositeView, InterpreterState, MutCompositeView, MutStateView,
        StateView,
    },
    errors::InterpreterResult,
    interpreter::utils::{is_signal_high, ConstPort},
    values::Value,
};
use calyx::ir::{self, Assignment, Component, Control, Guard, RRC};
use itertools::{peek_nth, Itertools, PeekNth};
use std::cell::Ref;
use std::collections::HashSet;
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

    fn deconstruct(self) -> InterpreterResult<InterpreterState<'outer>>;

    fn is_done(&self) -> bool;

    fn get_env(&self) -> StateView<'_, 'outer>;

    fn currently_executing_group(&self) -> Vec<&ir::Id>;

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer>;
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

    fn deconstruct(self) -> InterpreterResult<InterpreterState<'outer>> {
        Ok(self.env)
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

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        (&mut self.env).into()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        Ok(())
    }
}

pub enum EnableHolder<'a> {
    RefGroup(Ref<'a, ir::Group>),
    RefCombGroup(Ref<'a, ir::CombGroup>),
    BorrowGrp(&'a ir::Group),
    BorrowCombGroup(&'a ir::CombGroup),
}

impl<'a> From<&'a ir::Group> for EnableHolder<'a> {
    fn from(grp: &'a ir::Group) -> Self {
        Self::BorrowGrp(grp)
    }
}

impl<'a> From<&'a ir::CombGroup> for EnableHolder<'a> {
    fn from(comb_grp: &'a ir::CombGroup) -> Self {
        Self::BorrowCombGroup(comb_grp)
    }
}

impl<'a> From<Ref<'a, ir::Group>> for EnableHolder<'a> {
    fn from(grp: Ref<'a, ir::Group>) -> Self {
        Self::RefGroup(grp)
    }
}

impl<'a> From<Ref<'a, ir::CombGroup>> for EnableHolder<'a> {
    fn from(comb_grp: Ref<'a, ir::CombGroup>) -> Self {
        Self::RefCombGroup(comb_grp)
    }
}

impl<'a> From<&'a ir::Enable> for EnableHolder<'a> {
    fn from(en: &'a ir::Enable) -> Self {
        Self::RefGroup(en.group.borrow())
    }
}

impl<'a> EnableHolder<'a> {
    fn assignments(&self) -> &[ir::Assignment] {
        match self {
            EnableHolder::RefGroup(x) => &x.assignments,
            EnableHolder::RefCombGroup(x) => &x.assignments,
            EnableHolder::BorrowGrp(x) => &x.assignments,
            EnableHolder::BorrowCombGroup(x) => &x.assignments,
        }
    }

    fn done_port(&self) -> Option<RRC<ir::Port>> {
        match self {
            EnableHolder::RefGroup(x) => Some(get_done_port(x)),
            EnableHolder::BorrowGrp(x) => Some(get_done_port(x)),
            EnableHolder::BorrowCombGroup(_)
            | EnableHolder::RefCombGroup(_) => None,
        }
    }

    fn go_port(&self) -> Option<RRC<ir::Port>> {
        match self {
            EnableHolder::RefGroup(x) => Some(get_go_port(x)),
            EnableHolder::BorrowGrp(x) => Some(get_go_port(x)),
            EnableHolder::BorrowCombGroup(_)
            | EnableHolder::RefCombGroup(_) => None,
        }
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
        mut env: InterpreterState<'outer>,
        continuous: &'a [Assignment],
    ) -> Self
    where
        E: Into<EnableHolder<'a>>,
    {
        let enable: EnableHolder = enable.into();

        if let Some(go) = enable.go_port() {
            env.insert(go, Value::bit_high())
        }

        let assigns = (
            enable.assignments().iter().cloned().collect_vec(),
            continuous.iter().cloned().collect_vec(),
        );
        let done = enable.done_port();
        let interp = AssignmentInterpreter::new_owned(env, done, assigns);
        Self {
            enable,
            group_name,
            interp,
        }
    }
}

impl<'a, 'outer> EnableInterpreter<'a, 'outer> {
    fn reset(mut self) -> InterpreterResult<InterpreterState<'outer>> {
        if let Some(go) = self.enable.go_port() {
            self.interp.get_mut_env().insert(go, Value::bit_low())
        }

        self.interp.reset()
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

    fn deconstruct(self) -> InterpreterResult<InterpreterState<'outer>> {
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
                let res = run_and_deconstruct!(interp)?;
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

    fn deconstruct(self) -> InterpreterResult<InterpreterState<'outer>> {
        self.env.ok_or(InterpreterError::InvalidSeqState)
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
        let mut env = env.force_fork();
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

    fn deconstruct(self) -> InterpreterResult<InterpreterState<'outer>> {
        assert!(self.interpreters.iter().all(|x| x.is_done()));
        let envs = self
            .interpreters
            .into_iter()
            .map(ControlInterpreter::deconstruct)
            .collect::<InterpreterResult<Vec<InterpreterState<'outer>>>>()?;

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
        let cond_port: ConstPort = ctrl_if.port.as_ptr();

        let (cond, branch_interp) = if let Some(cond) = &ctrl_if.cond {
            (
                Some(EnableInterpreter::new(
                    cond.borrow(),
                    Some(cond.borrow().name().clone()),
                    env,
                    continuous_assigns,
                )),
                None,
            )
        } else {
            let grp = if is_signal_high(env.get_from_port(cond_port)) {
                &ctrl_if.tbranch
            } else {
                &ctrl_if.fbranch
            };
            (
                None,
                Some(ControlInterpreter::new(
                    grp,
                    env,
                    continuous_assigns,
                    input_ports.clone(),
                )),
            )
        };

        Self {
            port: cond_port,
            cond,
            tbranch: &ctrl_if.tbranch,
            fbranch: &ctrl_if.fbranch,
            branch_interp,
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
                    let env = i.deconstruct()?;
                    branch = ControlInterpreter::new(
                        self.tbranch,
                        env,
                        self.continuous_assignments,
                        Rc::clone(&self.input_ports),
                    );
                } else {
                    let env = i.deconstruct()?;
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

    fn deconstruct(self) -> InterpreterResult<InterpreterState<'outer>> {
        self.branch_interp
            .ok_or(InterpreterError::InvalidIfState)?
            .deconstruct()
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
            unreachable!("Invalid internal state for IfInterpreter. It is neither evaluating the conditional or the branch. This indicates an error in the internal state transition.")
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

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        if let Some(cond) = &mut self.cond {
            cond.get_mut_env()
        } else if let Some(branch) = &mut self.branch_interp {
            branch.get_mut_env()
        } else {
            unreachable!("Invalid internal state for IfInterpreter. It is neither evaluating the conditional or the branch. This indicates an error in the internal state transition.")
        }
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        match (&mut self.cond, &mut self.branch_interp) {
            (None, Some(i)) => i.converge(),
            (Some(i), None) => i.converge(),
            _ => unreachable!("Invalid internal state for IfInterpreter. It is neither evaluating the conditional or the branch. This indicates an error in the internal state transition."),
        }
    }
}
pub struct WhileInterpreter<'a, 'outer> {
    port: ConstPort,
    cond: Option<Ref<'a, ir::CombGroup>>,
    body: &'a Control,
    continuous_assignments: &'a [ir::Assignment],
    cond_interp: Option<EnableInterpreter<'a, 'outer>>,
    body_interp: Option<ControlInterpreter<'a, 'outer>>,
    input_ports: Rc<HashSet<*const ir::Port>>,
    terminal_env: Option<InterpreterState<'outer>>,
}

impl<'a, 'outer> WhileInterpreter<'a, 'outer> {
    pub fn new(
        ctrl_while: &'a ir::While,
        env: InterpreterState<'outer>,
        continuous_assignments: &'a [Assignment],
        input_ports: Rc<HashSet<*const ir::Port>>,
    ) -> Self {
        let port: ConstPort = ctrl_while.port.as_ptr();
        let cond_interp;
        let body_interp;
        let terminal_env;

        if let Some(cond) = &ctrl_while.cond {
            cond_interp = Some(EnableInterpreter::new(
                cond.borrow(),
                Some(cond.borrow().name().clone()),
                env,
                continuous_assignments,
            ));
            terminal_env = None;
            body_interp = None;
        } else if is_signal_high(env.get_from_port(port)) {
            body_interp = Some(ControlInterpreter::new(
                &ctrl_while.body,
                env,
                continuous_assignments,
                input_ports.clone(),
            ));
            terminal_env = None;
            cond_interp = None;
        } else {
            terminal_env = Some(env);
            body_interp = None;
            cond_interp = None;
        }

        Self {
            port,
            cond: ctrl_while.cond.as_ref().map(|x| x.borrow()),
            body: &ctrl_while.body,
            continuous_assignments,
            input_ports,
            cond_interp,
            body_interp,
            terminal_env,
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
                        ci.deconstruct()?,
                        self.continuous_assignments,
                        Rc::clone(&self.input_ports),
                    );
                    self.body_interp = Some(body_interp)
                } else {
                    self.terminal_env = Some(ci.deconstruct()?)
                }
            } else {
                ci.step()?
            }
        } else if let Some(bi) = &mut self.body_interp {
            if !bi.is_done() {
                bi.step()?
            } else {
                let bi = self.body_interp.take().unwrap();
                let env = bi.deconstruct()?;

                if let Some(cond) = &self.cond {
                    let cond_interp = EnableInterpreter::new(
                        Ref::clone(cond),
                        Some(cond.name().clone()),
                        env,
                        self.continuous_assignments,
                    );
                    self.cond_interp = Some(cond_interp)
                } else if is_signal_high(env.get_from_port(self.port)) {
                    self.body_interp = Some(ControlInterpreter::new(
                        self.body,
                        env,
                        self.continuous_assignments,
                        Rc::clone(&self.input_ports),
                    ));
                } else {
                    self.terminal_env = Some(env);
                }
            }
        } else if self.terminal_env.is_some() {
        } else {
            unreachable!("Invalid internal state for WhileInterpreter. It is neither evaluating the condition, nor the body, but it is also not finished executing. This indicates an error in the internal state transition and should be reported.")
        }
        Ok(())
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState<'outer>> {
        self.terminal_env.ok_or(InterpreterError::InvalidIfState)
    }

    fn is_done(&self) -> bool {
        self.terminal_env.is_some()
    }

    fn get_env(&self) -> StateView<'_, 'outer> {
        if let Some(cond) = &self.cond_interp {
            cond.get_env()
        } else if let Some(body) = &self.body_interp {
            body.get_env()
        } else if let Some(env) = &self.terminal_env {
            env.into()
        } else {
            unreachable!("Invalid internal state for WhileInterpreter. It is neither evaluating the condition, nor the body, but it is also not finished executing. This indicates an error in the internal state transition and should be reported.")
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

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        if let Some(cond) = &mut self.cond_interp {
            cond.get_mut_env()
        } else if let Some(body) = &mut self.body_interp {
            body.get_mut_env()
        } else if let Some(term) = &mut self.terminal_env {
            term.into()
        } else {
            unreachable!("Invalid internal state for WhileInterpreter. It is neither evaluating the condition, nor the body, but it is also not finished executing. This indicates an error in the internal state transition and should be reported.")
        }
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        if let Some(cond) = &mut self.cond_interp {
            cond.converge()
        } else if let Some(body) = &mut self.body_interp {
            body.converge()
        } else if let Some(_term) = &mut self.terminal_env {
            Ok(())
        } else {
            unreachable!("Invalid internal state for WhileInterpreter. It is neither evaluating the condition, nor the body, but it is also not finished executing. This indicates an error in the internal state transition and should be reported.")
        }
    }
}
pub struct InvokeInterpreter<'a, 'outer> {
    invoke: &'a ir::Invoke,
    assign_interp: AssignmentInterpreter<'a, 'outer>,
}

impl<'a, 'outer> InvokeInterpreter<'a, 'outer> {
    pub fn new(
        invoke: &'a ir::Invoke,
        mut env: InterpreterState<'outer>,
        continuous_assignments: &'a [Assignment],
    ) -> Self {
        let mut assignment_vec: Vec<Assignment> = vec![];
        let comp_cell = invoke.comp.borrow();

        //first connect the inputs (from connection -> input)
        for (input_port, connection) in &invoke.inputs {
            let comp_input_port = comp_cell.get(input_port);
            assignment_vec.push(Assignment {
                dst: comp_input_port,
                src: Rc::clone(connection),
                guard: Guard::default().into(),
            });
        }

        //second connect the output ports (from output -> connection)
        for (output_port, connection) in &invoke.outputs {
            let comp_output_port = comp_cell.get(output_port);
            assignment_vec.push(Assignment {
                dst: Rc::clone(connection),
                src: comp_output_port,
                guard: Guard::default().into(),
            })
        }

        let go_port = comp_cell.get_with_attr("go");
        // insert one into the go_port
        // should probably replace with an actual assignment from a constant one
        env.insert(go_port, Value::bit_high());

        let comp_done_port = comp_cell.get_with_attr("done");
        let interp = AssignmentInterpreter::new_owned_grp(
            env,
            comp_done_port.into(),
            (assignment_vec, continuous_assignments.iter()),
        );

        Self {
            invoke,
            assign_interp: interp,
        }
    }
}

impl<'a, 'outer> Interpreter<'outer> for InvokeInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        self.assign_interp.step()
    }

    fn run(&mut self) -> InterpreterResult<()> {
        self.assign_interp.run()
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState<'outer>> {
        let mut env = self.assign_interp.reset()?;

        // set go low
        let go_port = self.invoke.comp.borrow().get_with_attr("go");
        // insert one into the go_port
        // should probably replace with an actual assignment from a constant one
        env.insert(go_port, Value::bit_low());

        Ok(env)
    }

    fn is_done(&self) -> bool {
        self.assign_interp.is_deconstructable()
    }

    fn get_env(&self) -> StateView<'_, 'outer> {
        self.assign_interp.get_env().into()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        vec![]
    }

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        self.assign_interp.get_mut_env().into()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        self.assign_interp.step_convergence()
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
            Control::Invoke(i) => Self::Invoke(Box::new(
                InvokeInterpreter::new(i, env, continuous_assignments),
            )),
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

    fn deconstruct(self) -> InterpreterResult<InterpreterState<'outer>> {
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
}

impl<'a, 'outer> StructuralInterpreter<'a, 'outer> {
    pub fn from_component(
        comp: &'a Component,
        env: InterpreterState<'outer>,
    ) -> Self {
        let comp_sig = comp.signature.borrow();
        let done_port = comp_sig.get_with_attr("done");
        let done_raw = done_port.as_raw();
        let continuous_assignments = &comp.continuous_assignments;

        let interp = AssignmentInterpreter::new(
            env,
            Some(done_port),
            (std::iter::empty(), continuous_assignments.iter()),
        );

        Self {
            interp,
            continuous: continuous_assignments,
            done_port: done_raw,
        }
    }
}

impl<'a, 'outer> Interpreter<'outer> for StructuralInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        self.interp.step()
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState<'outer>> {
        let final_env = self.interp.deconstruct()?;
        finish_interpretation(
            final_env,
            Some(self.done_port),
            self.continuous.iter(),
        )
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

    fn get_mut_env(&mut self) -> MutStateView<'_, 'outer> {
        self.interp.get_mut_env().into()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        self.interp.step_convergence()
    }
}
