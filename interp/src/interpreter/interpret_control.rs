//! Inteprets a control in a component.

use super::interpret_group::{finish_group_interpretation, interpret_group};
use crate::environment::Environment;
use calyx::{errors::FutilResult, ir};

pub fn interpret_control(
    ctrl: &ir::Control,
    continuous_assignments: &[ir::Assignment],
    env: Environment,
) -> FutilResult<Environment> {
    if let ir::Control::Seq(ir::Seq { stmts, .. }) = ctrl {
        if stmts.len() == 1 {
            match &stmts[0] {
                ir::Control::Enable(e) => interpret_group(
                    &e.group.borrow(),
                    continuous_assignments,
                    env,
                ),
                _ => interpret_control_inner(ctrl, continuous_assignments, env),
            }
        } else {
            interpret_control_inner(ctrl, continuous_assignments, env)
        }
    } else {
        interpret_control_inner(ctrl, continuous_assignments, env)
    }
}

/// Helper function to evaluate control
fn interpret_control_inner(
    ctrl: &ir::Control,
    continuous_assignments: &[ir::Assignment],
    env: Environment,
) -> FutilResult<Environment> {
    match ctrl {
        ir::Control::Seq(s) => eval_seq(s, continuous_assignments, env),
        ir::Control::Par(p) => eval_par(p, continuous_assignments, env),
        ir::Control::If(i) => eval_if(i, continuous_assignments, env),
        ir::Control::While(w) => eval_while(w, continuous_assignments, env),
        ir::Control::Invoke(i) => eval_invoke(i, continuous_assignments, env),
        ir::Control::Enable(e) => eval_enable(e, continuous_assignments, env),
        ir::Control::Empty(e) => eval_empty(e, continuous_assignments, env),
    }
}

/// Interpret Seq
fn eval_seq(
    s: &ir::Seq,
    continuous_assignments: &[ir::Assignment],
    mut env: Environment,
) -> FutilResult<Environment> {
    for stmt in &s.stmts {
        env = interpret_control_inner(stmt, continuous_assignments, env)?;
    }
    Ok(env)
}

/// Interpret Par
/// at the moment behaves like seq
fn eval_par(
    _p: &ir::Par,
    _continuous_assignments: &[ir::Assignment],
    mut _env: Environment,
) -> FutilResult<Environment> {
    // for stmt in &p.stmts {
    //     env = interpret_control(stmt, comp.clone(), env)?;
    // }
    todo!()
}

/// Interpret If
fn eval_if(
    i: &ir::If,
    continuous_assignments: &[ir::Assignment],
    mut env: Environment,
) -> FutilResult<Environment> {
    env = interpret_group(&i.cond.borrow(), continuous_assignments, env)?;
    let cond_flag = env.get_from_port(&i.port.borrow()).as_u64();
    env = finish_group_interpretation(
        &i.cond.borrow(),
        continuous_assignments,
        env,
    )?;

    if cond_flag == 0 {
        env = interpret_control_inner(&i.fbranch, continuous_assignments, env)?;
    } else {
        env = interpret_control_inner(&i.tbranch, continuous_assignments, env)?;
    }
    Ok(env)
}

/// Interpret While
// /// The loop statement is similar to the conditional. It enables
// cond_group and uses port_name as the conditional value. When the
// value is high, it executes body_stmt and recomputes the conditional
// using cond_group.
fn eval_while(
    w: &ir::While,
    continuous_assignments: &[ir::Assignment],
    mut env: Environment,
) -> FutilResult<Environment> {
    env = interpret_group(&w.cond.borrow(), continuous_assignments, env)?;

    let cond_val = env.get_from_port(&w.port.borrow()).as_u64();
    env = finish_group_interpretation(
        &w.cond.borrow(),
        continuous_assignments,
        env,
    )?;

    if cond_val == 1 {
        return eval_while(
            w,
            continuous_assignments,
            interpret_control_inner(&w.body, continuous_assignments, env)?,
        );
    }
    Ok(env)
}

/// Interpret Invoke
/// TODO
#[allow(clippy::unnecessary_wraps)]
fn eval_invoke(
    _i: &ir::Invoke,
    _continuous_assignments: &[ir::Assignment],
    _env: Environment,
) -> FutilResult<Environment> {
    todo!()
}

/// Interpret Enable
fn eval_enable(
    e: &ir::Enable,
    continuous_assignments: &[ir::Assignment],
    mut env: Environment,
) -> FutilResult<Environment> {
    env = interpret_group(&e.group.borrow(), continuous_assignments, env)?;
    finish_group_interpretation(&e.group.borrow(), continuous_assignments, env)
}

/// Interpret Empty
#[allow(clippy::unnecessary_wraps)]
fn eval_empty(
    _e: &ir::Empty,
    _continuous_assignments: &[ir::Assignment],
    env: Environment,
) -> FutilResult<Environment> {
    Ok(env)
}
