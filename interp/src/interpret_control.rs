//! Inteprets a control in a component.

use super::{environment::Environment, interpret_group::GroupInterpreter};
use calyx::{
    errors::{Error, FutilResult},
    ir,
};
use std::rc::Rc;

/// Interpret a control.
/// TODO: Attributes, implementation for some component variants
pub struct ControlInterpreter {
    /// The environment
    pub environment: Environment,

    /// The component the control belongs to
    // XX(2/25 meeting): we might not need this? all the information is in the IR
    pub component: String,

    /// The control to interpret
    pub control: ir::RRC<ir::Control>,
}

impl ControlInterpreter {
    /// Construct ControlInterpreter
    /// env : Initial environment
    /// comp : Name of the component the Control is from
    /// ctrl : The control to interpret
    pub fn init(
        env: Environment,
        comp: String,
        ctrl: ir::RRC<ir::Control>,
    ) -> Self {
        Self {
            environment: env,
            component: comp,
            control: ctrl,
        }
    }

    /// Interpret this control
    pub fn interpret(self) -> FutilResult<Environment> {
        eval_control(
            &(*self.control.borrow()),
            self.component.clone(),
            self.environment,
        )
    }
}

/// Helper function to evaluate control
fn eval_control(
    ctrl: &ir::Control,
    comp: String,
    env: Environment,
) -> FutilResult<Environment> {
    let result = match ctrl {
        ir::Control::Seq(s) => eval_seq(s, comp, env),
        ir::Control::Par(p) => eval_par(p, comp, env),
        ir::Control::If(i) => eval_if(i, comp, env),
        ir::Control::While(w) => eval_while(w, comp, env),
        ir::Control::Invoke(i) => eval_invoke(i, comp, env),
        ir::Control::Enable(e) => eval_enable(e, comp, env),
        ir::Control::Empty(e) => eval_empty(e, comp, env),
    };
    result
}

/// Interpret Seq
fn eval_seq(
    s: &ir::Seq,
    comp: String,
    mut env: Environment,
) -> FutilResult<Environment> {
    for stmt in &s.stmts {
        env = eval_control(stmt, comp.clone(), env)?;
    }
    Ok(env)
}

/// Interpret Par
/// TODO
fn eval_par(
    p: &ir::Par,
    comp: String,
    mut env: Environment,
) -> FutilResult<Environment> {
    for stmt in &p.stmts {
        env = eval_control(stmt, comp.clone(), env)?;
    }
    Ok(env)
}

/// Interpret If
/// TODO
fn eval_if(
    i: &ir::If,
    comp: String,
    env: Environment,
) -> FutilResult<Environment> {
    Ok(env)
}

/// Interpret While
/// TODO
fn eval_while(
    w: &ir::While,
    comp: String,
    env: Environment,
) -> FutilResult<Environment> {
    Ok(env)
}

/// Interpret Invoke
/// TODO
fn eval_invoke(
    i: &ir::Invoke,
    comp: String,
    env: Environment,
) -> FutilResult<Environment> {
    Ok(env)
}

/// Interpret Enable
fn eval_enable(
    e: &ir::Enable,
    comp: String,
    env: Environment,
) -> FutilResult<Environment> {
    let gp = Rc::clone(&(e.group));

    //println!("Enable group {:?}", e.group.borrow().name);

    // TODO
    let gi = GroupInterpreter::init(comp.clone(), gp, env);
    gi.interpret()
}

/// Interpret Empty
fn eval_empty(
    _e: &ir::Empty,
    _comp: String,
    env: Environment,
) -> FutilResult<Environment> {
    Ok(env)
}
