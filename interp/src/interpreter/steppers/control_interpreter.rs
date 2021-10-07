use super::super::utils::{get_done_port, get_go_port};
use super::AssignmentInterpreter;
use crate::errors::InterpreterError;
use crate::interpreter::interpret_group::finish_interpretation;
use crate::interpreter_ir as iir;
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
use calyx::ir::{self, Assignment, Guard, RRC};
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

    fn currently_executing_group(&self) -> Vec<&ir::Id>;

    fn get_mut_env(&mut self) -> MutStateView<'_>;
}

pub struct EmptyInterpreter {
    pub(super) env: InterpreterState,
}

impl EmptyInterpreter {
    pub fn new(env: InterpreterState) -> Self {
        Self { env }
    }
}

impl Interpreter for EmptyInterpreter {
    fn step(&mut self) -> InterpreterResult<()> {
        Ok(())
    }

    fn run(&mut self) -> InterpreterResult<()> {
        Ok(())
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState> {
        Ok(self.env)
    }

    fn is_done(&self) -> bool {
        true
    }

    fn get_env(&self) -> StateView<'_> {
        (&self.env).into()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        vec![]
    }

    fn get_mut_env(&mut self) -> MutStateView<'_> {
        (&mut self.env).into()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub enum EnableHolder {
    Group(RRC<ir::Group>),
    CombGroup(RRC<ir::CombGroup>),
    Vec(Rc<Vec<ir::Assignment>>),
}

impl From<RRC<ir::Group>> for EnableHolder {
    fn from(gr: RRC<ir::Group>) -> Self {
        Self::Group(gr)
    }
}

impl From<RRC<ir::CombGroup>> for EnableHolder {
    fn from(cb: RRC<ir::CombGroup>) -> Self {
        Self::CombGroup(cb)
    }
}

impl From<&RRC<ir::Group>> for EnableHolder {
    fn from(gr: &RRC<ir::Group>) -> Self {
        Self::Group(Rc::clone(gr))
    }
}

impl From<&RRC<ir::CombGroup>> for EnableHolder {
    fn from(cb: &RRC<ir::CombGroup>) -> Self {
        Self::CombGroup(Rc::clone(cb))
    }
}

impl From<Vec<ir::Assignment>> for EnableHolder {
    fn from(v: Vec<ir::Assignment>) -> Self {
        Self::Vec(Rc::new(v))
    }
}

impl From<&iir::Enable> for EnableHolder {
    fn from(en: &iir::Enable) -> Self {
        (&en.group).into()
    }
}

impl EnableHolder {
    fn done_port(&self) -> Option<RRC<ir::Port>> {
        match self {
            EnableHolder::Group(g) => Some(get_done_port(&g.borrow())),
            EnableHolder::CombGroup(_) | EnableHolder::Vec(_) => None,
        }
    }

    fn go_port(&self) -> Option<RRC<ir::Port>> {
        match self {
            EnableHolder::Group(g) => Some(get_go_port(&g.borrow())),
            EnableHolder::CombGroup(_) | EnableHolder::Vec(_) => None,
        }
    }
}

pub struct EnableInterpreter {
    enable: EnableHolder,
    group_name: Option<ir::Id>,
    interp: AssignmentInterpreter,
    _continuous_assignments: iir::ContinuousAssignments,
}

impl EnableInterpreter {
    pub fn new<E>(
        enable: E,
        group_name: Option<ir::Id>,
        mut env: InterpreterState,
        continuous: &iir::ContinuousAssignments,
    ) -> Self
    where
        E: Into<EnableHolder>,
    {
        let enable: EnableHolder = enable.into();

        if let Some(go) = enable.go_port() {
            env.insert(go, Value::bit_high())
        }

        let assigns = enable.clone();
        let done = enable.done_port();
        let interp = AssignmentInterpreter::new(env, done, assigns, continuous);
        Self {
            enable,
            group_name,
            interp,
            _continuous_assignments: Rc::clone(continuous),
        }
    }
}

impl EnableInterpreter {
    fn reset(mut self) -> InterpreterResult<InterpreterState> {
        if let Some(go) = self.enable.go_port() {
            self.interp.get_mut_env().insert(go, Value::bit_low())
        }

        self.interp.reset()
    }
    fn get<P: AsRaw<ir::Port>>(&self, port: P) -> &Value {
        self.interp.get(port)
    }
}

impl Interpreter for EnableInterpreter {
    fn step(&mut self) -> InterpreterResult<()> {
        self.interp.step()
    }

    fn run(&mut self) -> InterpreterResult<()> {
        self.interp.run()
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState> {
        self.reset()
    }

    fn is_done(&self) -> bool {
        self.interp.is_deconstructable()
    }

    fn get_env(&self) -> StateView<'_> {
        (self.interp.get_env()).into()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        if let Some(name) = &self.group_name {
            vec![name]
        } else {
            vec![]
        }
    }

    fn get_mut_env(&mut self) -> MutStateView<'_> {
        (self.interp.get_mut_env()).into()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        self.interp.step_convergence()
    }
}

pub struct SeqInterpreter {
    current_interpreter: Option<ControlInterpreter>,
    continuous_assignments: iir::ContinuousAssignments,
    env: Option<InterpreterState>,
    done_flag: bool,
    input_ports: Rc<HashSet<*const ir::Port>>,
    seq: Rc<iir::Seq>,
    seq_index: usize,
}
impl SeqInterpreter {
    pub fn new(
        seq: &Rc<iir::Seq>,
        env: InterpreterState,
        continuous_assigns: &iir::ContinuousAssignments,
        input_ports: Rc<HashSet<*const ir::Port>>,
    ) -> Self {
        Self {
            current_interpreter: None,
            continuous_assignments: Rc::clone(continuous_assigns),
            env: Some(env),
            done_flag: false,
            input_ports,
            seq: Rc::clone(seq),
            seq_index: 0,
        }
    }
}

impl Interpreter for SeqInterpreter {
    fn step(&mut self) -> InterpreterResult<()> {
        if self.current_interpreter.is_none()
            && self.seq_index < self.seq.stmts.len()
        // There is more to execute, make new interpreter
        {
            self.current_interpreter = ControlInterpreter::new(
                &self.seq.stmts[self.seq_index],
                self.env.take().unwrap(),
                &self.continuous_assignments,
                Rc::clone(&self.input_ports),
            )
            .into();
            self.seq_index += 1;
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
        } else if self.seq_index >= self.seq.stmts.len()
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

    fn deconstruct(self) -> InterpreterResult<InterpreterState> {
        self.env.ok_or(InterpreterError::InvalidSeqState)
    }

    fn get_env(&self) -> StateView<'_> {
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

    fn get_mut_env(&mut self) -> MutStateView<'_> {
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

pub struct ParInterpreter {
    _par: Rc<iir::Par>,
    interpreters: Vec<ControlInterpreter>,
    in_state: InterpreterState,
    input_ports: Rc<HashSet<*const ir::Port>>,
}

impl ParInterpreter {
    pub fn new(
        par: &Rc<iir::Par>,
        mut env: InterpreterState,
        continuous_assigns: &iir::ContinuousAssignments,
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
            _par: Rc::clone(par),
        }
    }
}

impl Interpreter for ParInterpreter {
    fn step(&mut self) -> InterpreterResult<()> {
        for i in &mut self.interpreters {
            i.step()?;
        }
        Ok(())
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState> {
        assert!(self.interpreters.iter().all(|x| x.is_done()));
        let envs = self
            .interpreters
            .into_iter()
            .map(ControlInterpreter::deconstruct)
            .collect::<InterpreterResult<Vec<InterpreterState>>>()?;

        self.in_state.merge_many(envs, &self.input_ports)
    }

    fn is_done(&self) -> bool {
        self.interpreters.iter().all(|x| x.is_done())
    }

    fn get_env(&self) -> StateView<'_> {
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

    fn get_mut_env(&mut self) -> MutStateView<'_> {
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
pub struct IfInterpreter {
    port: ConstPort,
    cond: Option<EnableInterpreter>,
    tbranch: iir::Control,
    fbranch: iir::Control,
    branch_interp: Option<ControlInterpreter>,
    continuous_assignments: iir::ContinuousAssignments,
    input_ports: Rc<HashSet<*const ir::Port>>,
}

impl IfInterpreter {
    pub fn new(
        ctrl_if: &Rc<iir::If>,
        env: InterpreterState,
        continuous_assigns: &iir::ContinuousAssignments,
        input_ports: Rc<HashSet<*const ir::Port>>,
    ) -> Self {
        let cond_port: ConstPort = ctrl_if.port.as_ptr();

        let (cond, branch_interp) = if let Some(cond) = &ctrl_if.cond {
            (
                Some(EnableInterpreter::new(
                    Rc::clone(cond),
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
            tbranch: ctrl_if.tbranch.clone(),
            fbranch: ctrl_if.fbranch.clone(),
            branch_interp,
            continuous_assignments: Rc::clone(continuous_assigns),
            input_ports,
        }
    }
}

impl Interpreter for IfInterpreter {
    fn step(&mut self) -> InterpreterResult<()> {
        if let Some(i) = &mut self.cond {
            if i.is_done() {
                let i = self.cond.take().unwrap();
                let branch;
                #[allow(clippy::branches_sharing_code)]
                if is_signal_high(i.get(self.port)) {
                    let env = i.deconstruct()?;
                    branch = ControlInterpreter::new(
                        &self.tbranch,
                        env,
                        &self.continuous_assignments,
                        Rc::clone(&self.input_ports),
                    );
                } else {
                    let env = i.deconstruct()?;
                    branch = ControlInterpreter::new(
                        &self.fbranch,
                        env,
                        &self.continuous_assignments,
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

    fn deconstruct(self) -> InterpreterResult<InterpreterState> {
        self.branch_interp
            .ok_or(InterpreterError::InvalidIfState)?
            .deconstruct()
    }

    fn is_done(&self) -> bool {
        self.cond.is_none()
            && self.branch_interp.is_some()
            && self.branch_interp.as_ref().unwrap().is_done()
    }

    fn get_env(&self) -> StateView<'_> {
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

    fn get_mut_env(&mut self) -> MutStateView<'_> {
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
pub struct WhileInterpreter {
    port: ConstPort,
    continuous_assignments: iir::ContinuousAssignments,
    cond_interp: Option<EnableInterpreter>,
    body_interp: Option<ControlInterpreter>,
    input_ports: Rc<HashSet<*const ir::Port>>,
    terminal_env: Option<InterpreterState>,
    wh: Rc<iir::While>,
}

impl WhileInterpreter {
    pub fn new(
        ctrl_while: &Rc<iir::While>,
        env: InterpreterState,
        continuous_assignments: &iir::ContinuousAssignments,
        input_ports: Rc<HashSet<*const ir::Port>>,
    ) -> Self {
        let port: ConstPort = ctrl_while.port.as_ptr();
        let cond_interp;
        let body_interp;
        let terminal_env;

        if let Some(cond) = &ctrl_while.cond {
            cond_interp = Some(EnableInterpreter::new(
                cond,
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
            continuous_assignments: Rc::clone(continuous_assignments),
            input_ports,
            cond_interp,
            body_interp,
            terminal_env,
            wh: Rc::clone(ctrl_while),
        }
    }
}

impl Interpreter for WhileInterpreter {
    fn step(&mut self) -> InterpreterResult<()> {
        if let Some(ci) = &mut self.cond_interp {
            if ci.is_done() {
                let ci = self.cond_interp.take().unwrap();
                if is_signal_high(ci.get(self.port)) {
                    let body_interp = ControlInterpreter::new(
                        &self.wh.body,
                        ci.deconstruct()?,
                        &self.continuous_assignments,
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

                if let Some(cond) = &self.wh.cond {
                    let cond_interp = EnableInterpreter::new(
                        cond,
                        Some(cond.borrow().name().clone()),
                        env,
                        &self.continuous_assignments,
                    );
                    self.cond_interp = Some(cond_interp)
                } else if is_signal_high(env.get_from_port(self.port)) {
                    self.body_interp = Some(ControlInterpreter::new(
                        &self.wh.body,
                        env,
                        &self.continuous_assignments,
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

    fn deconstruct(self) -> InterpreterResult<InterpreterState> {
        self.terminal_env.ok_or(InterpreterError::InvalidIfState)
    }

    fn is_done(&self) -> bool {
        self.terminal_env.is_some()
    }

    fn get_env(&self) -> StateView<'_> {
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

    fn get_mut_env(&mut self) -> MutStateView<'_> {
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
pub struct InvokeInterpreter {
    invoke: Rc<iir::Invoke>,
    assign_interp: AssignmentInterpreter,
}

impl InvokeInterpreter {
    pub fn new(
        invoke: &Rc<iir::Invoke>,
        mut env: InterpreterState,
        continuous_assignments: &iir::ContinuousAssignments,
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
        let interp = AssignmentInterpreter::new(
            env,
            comp_done_port.into(),
            assignment_vec,
            continuous_assignments,
        );

        Self {
            invoke: Rc::clone(invoke),
            assign_interp: interp,
        }
    }
}

impl Interpreter for InvokeInterpreter {
    fn step(&mut self) -> InterpreterResult<()> {
        self.assign_interp.step()
    }

    fn run(&mut self) -> InterpreterResult<()> {
        self.assign_interp.run()
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState> {
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

    fn get_env(&self) -> StateView<'_> {
        self.assign_interp.get_env().into()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        vec![]
    }

    fn get_mut_env(&mut self) -> MutStateView<'_> {
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

pub enum ControlInterpreter {
    Empty(Box<EmptyInterpreter>),
    Enable(Box<EnableInterpreter>),
    Seq(Box<SeqInterpreter>),
    Par(Box<ParInterpreter>),
    If(Box<IfInterpreter>),
    While(Box<WhileInterpreter>),
    Invoke(Box<InvokeInterpreter>),
}

impl ControlInterpreter {
    pub fn new(
        control: &iir::Control,
        env: InterpreterState,
        continuous_assignments: &iir::ContinuousAssignments,
        input_ports: Rc<HashSet<*const ir::Port>>,
    ) -> Self {
        match control {
            iir::Control::Seq(s) => Self::Seq(Box::new(SeqInterpreter::new(
                s,
                env,
                continuous_assignments,
                input_ports,
            ))),
            iir::Control::Par(par) => Self::Par(Box::new(ParInterpreter::new(
                par,
                env,
                continuous_assignments,
                input_ports,
            ))),
            iir::Control::If(i) => Self::If(Box::new(IfInterpreter::new(
                i,
                env,
                continuous_assignments,
                input_ports,
            ))),
            iir::Control::While(w) => {
                Self::While(Box::new(WhileInterpreter::new(
                    w,
                    env,
                    continuous_assignments,
                    input_ports,
                )))
            }
            iir::Control::Invoke(i) => Self::Invoke(Box::new(
                InvokeInterpreter::new(i, env, continuous_assignments),
            )),
            iir::Control::Enable(e) => {
                Self::Enable(Box::new(EnableInterpreter::new(
                    &**e,
                    Some(e.group.borrow().name().clone()),
                    env,
                    continuous_assignments,
                )))
            }
            iir::Control::Empty(_) => {
                Self::Empty(Box::new(EmptyInterpreter::new(env)))
            }
        }
    }
}

impl Interpreter for ControlInterpreter {
    fn step(&mut self) -> InterpreterResult<()> {
        control_match!(self, i, i.step())
    }

    fn run(&mut self) -> InterpreterResult<()> {
        control_match!(self, i, i.run())
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState> {
        control_match!(self, i, i.deconstruct())
    }

    fn is_done(&self) -> bool {
        control_match!(self, i, i.is_done())
    }

    fn get_env(&self) -> StateView<'_> {
        control_match!(self, i, i.get_env())
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        control_match!(self, i, i.currently_executing_group())
    }

    fn get_mut_env(&mut self) -> MutStateView<'_> {
        control_match!(self, i, i.get_mut_env())
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        control_match!(self, i, i.converge())
    }
}

pub struct StructuralInterpreter {
    interp: AssignmentInterpreter,
    continuous: iir::ContinuousAssignments,
    done_port: ConstPort,
}

impl StructuralInterpreter {
    pub fn from_component(
        comp: &Rc<iir::Component>,
        env: InterpreterState,
    ) -> Self {
        let comp_sig = comp.signature.borrow();
        let done_port = comp_sig.get_with_attr("done");
        let done_raw = done_port.as_raw();
        let continuous = Rc::clone(&comp.continuous_assignments);
        let assigns: Vec<ir::Assignment> = vec![];

        let interp = AssignmentInterpreter::new(
            env,
            Some(done_port),
            assigns,
            &continuous,
        );

        Self {
            interp,
            continuous,
            done_port: done_raw,
        }
    }
}

impl Interpreter for StructuralInterpreter {
    fn step(&mut self) -> InterpreterResult<()> {
        self.interp.step()
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState> {
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

    fn get_env(&self) -> StateView<'_> {
        self.interp.get_env().into()
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        vec![]
    }

    fn get_mut_env(&mut self) -> MutStateView<'_> {
        self.interp.get_mut_env().into()
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        self.interp.step_convergence()
    }
}
