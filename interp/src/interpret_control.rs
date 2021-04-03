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

    /// The control
    pub control: ir::RRC<ir::Control>,
}

impl ControlInterpreter {
    /// Interpret this control
    pub fn interpret(self) -> FutilResult<Environment> {
        eval_control(&&*self.control.borrow(), self.environment)
    }
}

//. Evaluate control
fn eval_control(
    ctrl: &ir::Control,
    env: Environment,
) -> FutilResult<Environment> {
    let result = match ctrl {
        ir::Control::Seq(s) => eval_seq(s, env),
        ir::Control::Par(p) => eval_par(p, env),
        ir::Control::If(i) => eval_if(i, env),
        ir::Control::While(w) => eval_while(w, env),
        ir::Control::Invoke(i) => eval_invoke(i, env),
        ir::Control::Enable(e) => eval_enable(e, env),
        ir::Control::Empty(e) => eval_empty(e, env),
    };
    result
}

/// Interpret Seq
fn eval_seq(s: &ir::Seq, mut env: Environment) -> FutilResult<Environment> {
    for stmt in &s.stmts {
        println!("statement {:?} :", stmt);
        env = eval_control(stmt, env)?;
    }
    Ok(env)
}

/// Interpret Par
/// TODO
fn eval_par(p: &ir::Par, mut env: Environment) -> FutilResult<Environment> {
    for stmt in &p.stmts {
        env = eval_control(stmt, env)?;
    }
    Ok(env)
}

/// Interpret If
/// TODO
fn eval_if(i: &ir::If, env: Environment) -> FutilResult<Environment> {
    Ok(env)
}

/// Interpret While
/// TODO
fn eval_while(w: &ir::While, env: Environment) -> FutilResult<Environment> {
    Ok(env)
}

/// Interpret Invoke
/// TODO
fn eval_invoke(i: &ir::Invoke, env: Environment) -> FutilResult<Environment> {
    Ok(env)
}

/// Interpret Enable
fn eval_enable(e: &ir::Enable, env: Environment) -> FutilResult<Environment> {
    let gp = Rc::clone(&(e.group));

    println!("{:?}", e.group.borrow().name);

    // TODO
    // let gi =
    //     interpret_group::GroupInterpreter::init("main".to_string(), gp, env);

    Ok(env)

    //gi.interpret()
}

/// Interpret Empty
fn eval_empty(e: &ir::Empty, env: Environment) -> FutilResult<Environment> {
    Ok(env)
}
