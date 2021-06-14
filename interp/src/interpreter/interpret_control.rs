//! Inteprets a control in a component.

use super::interpret_group::interpret_group;
use crate::environment::Environment;
use calyx::{errors::FutilResult, ir};

/// Helper function to evaluate control
pub fn interpret_control(
    ctrl: &ir::Control,
    env: Environment,
) -> FutilResult<Environment> {
    match ctrl {
        ir::Control::Seq(s) => eval_seq(s, env),
        ir::Control::Par(p) => eval_par(p, env),
        ir::Control::If(i) => eval_if(i, env),
        ir::Control::While(w) => eval_while(w, env),
        ir::Control::Invoke(i) => eval_invoke(i, env),
        ir::Control::Enable(e) => eval_enable(e, env),
        ir::Control::Empty(e) => eval_empty(e, env),
    }
}

/// Interpret Seq
fn eval_seq(s: &ir::Seq, mut env: Environment) -> FutilResult<Environment> {
    for stmt in &s.stmts {
        env = interpret_control(stmt, env)?;
    }
    Ok(env)
}

/// Interpret Par
/// at the moment behaves like seq
fn eval_par(_p: &ir::Par, mut _env: Environment) -> FutilResult<Environment> {
    // for stmt in &p.stmts {
    //     env = interpret_control(stmt, comp.clone(), env)?;
    // }
    todo!()
}

/// Interpret If
fn eval_if(i: &ir::If, mut env: Environment) -> FutilResult<Environment> {
    env = interpret_group(&i.cond.borrow(), env).unwrap();

    if env.get_from_port(&i.port.borrow()).as_u64() == 0 {
        env = interpret_control(&i.fbranch, env).unwrap();
        return Ok(env);
    } else {
        env = interpret_control(&i.tbranch, env).unwrap();
        return Ok(env);
    }
}

/// Interpret While
// /// The loop statement is similar to the conditional. It enables
// cond_group and uses port_name as the conditional value. When the
// value is high, it executes body_stmt and recomputes the conditional
// using cond_group.
fn eval_while(w: &ir::While, mut env: Environment) -> FutilResult<Environment> {
    env = interpret_group(&w.cond.borrow(), env).unwrap();

    if env.get_from_port(&w.port.borrow()).as_u64() == 1 {
        env = interpret_control(&w.body, env).unwrap();
        return eval_while(w, env);
    }

    return Ok(env);
    // // currently ports don't update properly in mutli-cycle and runs into infinite loop
    // // count needs to be removed when the infinite loop problem is fixed
    // let mut count = 0;
    // while env.get_from_port(&comp, &w.port.borrow()) != 1 && count < 5 {
    //     env = interpret_control(&w.body, comp, env)?;
    //     env = interpret_group(&w.cond.borrow(), env, comp)?;
    //     // count needs to be remved
    //     count += 1;
    // }
    // Ok(env)
}

/// Interpret Invoke
/// TODO
#[allow(clippy::unnecessary_wraps)]
fn eval_invoke(_i: &ir::Invoke, _env: Environment) -> FutilResult<Environment> {
    todo!()
}

/// Interpret Enable
fn eval_enable(e: &ir::Enable, env: Environment) -> FutilResult<Environment> {
    interpret_group(&e.group.borrow(), env)
}

/// Interpret Empty
#[allow(clippy::unnecessary_wraps)]
fn eval_empty(_e: &ir::Empty, env: Environment) -> FutilResult<Environment> {
    Ok(env)
}
