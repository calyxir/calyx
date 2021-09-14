//! Inteprets a control in a component.

use super::interpret_group::{
    finish_comb_group_interpretation, finish_group_interpretation,
    interpret_comb_group, interpret_group,
};
use crate::environment::InterpreterState;
use crate::errors::InterpreterResult;
use calyx::ir;

/// Helper function to evaluate control
pub fn interpret_control(
    ctrl: &ir::Control,
    continuous_assignments: &[ir::Assignment],
    env: InterpreterState,
    comp: &ir::Component,
) -> InterpreterResult<InterpreterState> {
    match ctrl {
        ir::Control::Seq(s) => eval_seq(s, continuous_assignments, env, comp),
        ir::Control::Par(p) => eval_par(p, continuous_assignments, env, comp),
        ir::Control::If(i) => eval_if(i, continuous_assignments, env, comp),
        ir::Control::While(w) => {
            eval_while(w, continuous_assignments, env, comp)
        }
        ir::Control::Invoke(i) => eval_invoke(i, continuous_assignments, env),
        ir::Control::Enable(e) => eval_enable(e, continuous_assignments, env),
        ir::Control::Empty(e) => {
            eval_empty(e, continuous_assignments, env, comp)
        }
    }
}

/// Interpret Seq
fn eval_seq(
    s: &ir::Seq,
    continuous_assignments: &[ir::Assignment],
    mut env: InterpreterState,
    comp: &ir::Component,
) -> InterpreterResult<InterpreterState> {
    for stmt in &s.stmts {
        env = interpret_control(stmt, continuous_assignments, env, comp)?;
    }
    Ok(env)
}

/// Interpret Par

fn eval_par(
    p: &ir::Par,
    continuous_assignments: &[ir::Assignment],
    mut env: InterpreterState,
    comp: &ir::Component,
) -> InterpreterResult<InterpreterState> {
    //vector to keep track of all updated states
    let mut states = Vec::new();

    // evaluate each expression within the starter environment by forking from it
    for st in &p.stmts {
        states.push(interpret_control(
            st,
            continuous_assignments,
            env.fork(),
            comp,
        )?);
    }

    // states = &p.stmts.into_iter().map(|ctr| {
    //     interpret_control(ctr, continuous_assignments, env.fork(), comp)?
    // });

    //clock updates
    let mut tl = 0;

    //vector of smooshers from the states
    let mut smooshers = Vec::new();

    let mut final_st = env;

    //i do this using loops for clock updates
    for is in states {
        if is.clk > tl {
            tl = is.clk;
        }

        smooshers.push(is.pv_map);
    }

    final_st.pv_map = final_st.pv_map.merge_many(smooshers);
    final_st.clk = tl;

    Ok(final_st)
}

/// Interpret If
fn eval_if(
    i: &ir::If,
    continuous_assignments: &[ir::Assignment],
    mut env: InterpreterState,
    comp: &ir::Component,
) -> InterpreterResult<InterpreterState> {
    if let Some(comb) = &i.cond {
        env =
            interpret_comb_group(&comb.borrow(), continuous_assignments, env)?;
    }

    let cond_flag = env.get_from_port(&i.port.borrow()).as_u64();
    if let Some(comb) = &i.cond {
        env = finish_comb_group_interpretation(
            &comb.borrow(),
            continuous_assignments,
            env,
        )?;
    }

    let target = if cond_flag == 0 {
        &i.fbranch
    } else {
        &i.tbranch
    };

    interpret_control(target, continuous_assignments, env, comp)
}

/// Interpret While
// /// The loop statement is similar to the conditional. It enables
// cond_group and uses port_name as the conditional value. When the
// value is high, it executes body_stmt and recomputes the conditional
// using cond_group.
fn eval_while(
    w: &ir::While,
    continuous_assignments: &[ir::Assignment],
    mut env: InterpreterState,
    comp: &ir::Component,
) -> InterpreterResult<InterpreterState> {
    loop {
        if let Some(comb) = &w.cond {
            env = interpret_comb_group(
                &comb.borrow(),
                continuous_assignments,
                env,
            )?;
        }

        let cond_val = env.get_from_port(&w.port.borrow()).as_u64();

        if let Some(comb) = &w.cond {
            env = finish_comb_group_interpretation(
                &comb.borrow(),
                continuous_assignments,
                env,
            )?;
        }

        if cond_val == 0 {
            break;
        }

        env = interpret_control(&w.body, continuous_assignments, env, comp)?;
    }

    Ok(env)
}

/// Interpret Invoke
/// TODO
#[allow(clippy::unnecessary_wraps)]
fn eval_invoke(
    _i: &ir::Invoke,
    _continuous_assignments: &[ir::Assignment],
    _env: InterpreterState,
) -> InterpreterResult<InterpreterState> {
    todo!("invoke control operator")
}

/// Interpret Enable
fn eval_enable(
    e: &ir::Enable,
    continuous_assignments: &[ir::Assignment],
    mut env: InterpreterState,
) -> InterpreterResult<InterpreterState> {
    env = interpret_group(&e.group.borrow(), continuous_assignments, env)?;
    finish_group_interpretation(&e.group.borrow(), continuous_assignments, env)
}

/// Interpret Empty
#[allow(clippy::unnecessary_wraps)]
fn eval_empty(
    _e: &ir::Empty,
    _continuous_assignments: &[ir::Assignment],
    env: InterpreterState,
    _comp: &ir::Component,
) -> InterpreterResult<InterpreterState> {
    Ok(env)
}
