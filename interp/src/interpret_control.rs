//! Inteprets a control in a component.

use super::{
    environment::Environment, interpret_group::GroupInterpreter, interpreter,
};
use calyx::{errors::FutilResult, ir};
use std::rc::Rc;

/// Interpret a control.
/// TODO: Attributes, implementation for some component variants
pub struct ControlInterpreter {
    /// The environment
    pub environment: Environment,

    /// The component the control belongs to
    // XX(2/25 meeting): we might not need this? all the information is in the IR
    pub component: ir::Id,

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
        comp: ir::Id,
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
    comp: ir::Id,
    env: Environment,
) -> FutilResult<Environment> {
    match ctrl {
        ir::Control::Seq(s) => eval_seq(s, comp, env),
        ir::Control::Par(p) => eval_par(p, comp, env),
        ir::Control::If(i) => eval_if(i, comp, env),
        ir::Control::While(w) => eval_while(w, comp, env),
        ir::Control::Invoke(i) => eval_invoke(i, comp, env),
        ir::Control::Enable(e) => eval_enable(e, comp, env),
        ir::Control::Empty(e) => eval_empty(e, comp, env),
    }
}

/// Interpret Seq
fn eval_seq(
    s: &ir::Seq,
    comp: ir::Id,
    mut env: Environment,
) -> FutilResult<Environment> {
    for stmt in &s.stmts {
        env = eval_control(stmt, comp.clone(), env)?;
    }
    Ok(env)
}

/// Interpret Par
/// at the moment behaves like seq
fn eval_par(
    p: &ir::Par,
    comp: ir::Id,
    mut env: Environment,
) -> FutilResult<Environment> {
    for stmt in &p.stmts {
        env = eval_control(stmt, comp.clone(), env)?;
    }
    Ok(env)
}

/// Interpret If
fn eval_if(
    i: &ir::If,
    comp: ir::Id,
    mut env: Environment,
) -> FutilResult<Environment> {
    //first set the environment for cond
    env = interpreter::eval_group(i.cond.clone(), env, comp.clone())?;

    let cid = ir::Id::from(comp.clone());
    // if i.port is not high fbranch else tbranch
    if env.get_from_port(&cid, &i.port.borrow()) == 0 {
        env = eval_control(&i.fbranch, comp, env)?;
        Ok(env)
    } else {
        env = eval_control(&i.tbranch, comp, env)?;
        Ok(env)
    }
}

/// Interpret While
// /// The loop statement is similar to the conditional. It enables
// cond_group and uses port_name as the conditional value. When the
// value is high, it executes body_stmt and recomputes the conditional
// using cond_group.
fn eval_while(
    w: &ir::While,
    comp: ir::Id,
    mut env: Environment,
) -> FutilResult<Environment> {
    // currently ports don't update properly in mutli-cycle and runs into infinite loop
    // count needs to be removed when the infinite loop problem is fixed
    let mut count = 0;
    while env.get_from_port(&comp, &w.port.borrow()) != 1 && count < 5 {
        env = eval_control(&w.body, comp.clone(), env)?;
        env = interpreter::eval_group(w.cond.clone(), env, comp.clone())?;
        // count needs to be remved
        count += 1;
    }
    Ok(env)
}

/// Interpret Invoke
/// TODO
#[allow(clippy::unnecessary_wraps)]
fn eval_invoke(
    _i: &ir::Invoke,
    _comp: ir::Id,
    env: Environment,
) -> FutilResult<Environment> {
    Ok(env)
}

/// Interpret Enable
fn eval_enable(
    e: &ir::Enable,
    comp: ir::Id,
    env: Environment,
) -> FutilResult<Environment> {
    let gp = Rc::clone(&(e.group));

    //println!("Enable group {:?}", e.group.borrow().name);

    // TODO
    let gi = GroupInterpreter::init(comp, gp, env);
    gi.interpret()
}

/// Interpret Empty
#[allow(clippy::unnecessary_wraps)]
fn eval_empty(
    _e: &ir::Empty,
    _comp: ir::Id,
    env: Environment,
) -> FutilResult<Environment> {
    Ok(env)
}
