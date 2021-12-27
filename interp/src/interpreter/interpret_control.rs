//! Inteprets a control in a component.

use std::collections::HashSet;
use std::rc::Rc;

use calyx::ir::PortParent;

use super::interpret_group::{
    finish_comb_group_interpretation, finish_group_interpretation,
    interpret_comb_group, interpret_group, interpret_invoke,
};
use crate::environment::InterpreterState;
use crate::errors::InterpreterResult;

use crate::interpreter_ir as iir;
use crate::utils::AsRaw;

/// Helper function to evaluate control
pub fn interpret_control(
    ctrl: &iir::Control,
    continuous_assignments: &iir::ContinuousAssignments,
    env: InterpreterState,
    comp: &iir::Component,
) -> InterpreterResult<InterpreterState> {
    match ctrl {
        iir::Control::Seq(s) => eval_seq(s, continuous_assignments, env, comp),
        iir::Control::Par(p) => eval_par(p, continuous_assignments, env, comp),
        iir::Control::If(i) => eval_if(i, continuous_assignments, env, comp),
        iir::Control::While(w) => {
            eval_while(w, continuous_assignments, env, comp)
        }
        iir::Control::Invoke(i) => eval_invoke(i, continuous_assignments, env),
        iir::Control::Enable(e) => eval_enable(e, continuous_assignments, env),
        iir::Control::Empty(e) => {
            eval_empty(e, continuous_assignments, env, comp)
        }
    }
}

/// Interpret Seq
fn eval_seq(
    s: &iir::Seq,
    continuous_assignments: &iir::ContinuousAssignments,
    mut env: InterpreterState,
    comp: &iir::Component,
) -> InterpreterResult<InterpreterState> {
    for stmt in &s.stmts {
        env = interpret_control(stmt, continuous_assignments, env, comp)?;
    }
    Ok(env)
}

/// Interpret Par

fn eval_par(
    p: &iir::Par,
    continuous_assignments: &iir::ContinuousAssignments,
    mut env: InterpreterState,
    comp: &iir::Component,
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

    //i do this using loops for clock updates
    for is in states {
        if is.clk > tl {
            tl = is.clk;
        }

        smooshers.push(is.port_map);
    }

    let mut override_set: HashSet<_> = continuous_assignments
        .iter()
        .map(|a| a.dst.as_raw())
        .collect();

    for dest_cell in continuous_assignments.iter().map(|x| &x.dst) {
        let dst_ref = dest_cell.borrow();
        if let PortParent::Cell(c) = &dst_ref.parent {
            let cell = c.upgrade();
            let cell_ref = cell.borrow();
            for port in cell_ref.ports() {
                let port_ref = port.borrow();
                if let calyx::ir::Direction::Output = port_ref.direction {
                    override_set.insert(port.as_raw());
                }
            }
        }
    }

    env.port_map = env.port_map.merge_many(smooshers, &override_set)?;
    env.clk = tl;

    Ok(env)
}

/// Interpret If
fn eval_if(
    i: &iir::If,
    continuous_assignments: &iir::ContinuousAssignments,
    mut env: InterpreterState,
    comp: &iir::Component,
) -> InterpreterResult<InterpreterState> {
    if let Some(comb) = &i.cond {
        env =
            interpret_comb_group(Rc::clone(comb), continuous_assignments, env)?;
    }

    let cond_flag = env.get_from_port(&i.port.borrow()).as_bool();
    if let Some(comb) = &i.cond {
        env = finish_comb_group_interpretation(
            &comb.borrow(),
            continuous_assignments,
            env,
        )?;
    }

    let target = if !cond_flag { &i.fbranch } else { &i.tbranch };

    interpret_control(target, continuous_assignments, env, comp)
}

/// Interpret While
// /// The loop statement is similar to the conditional. It enables
// cond_group and uses port_name as the conditional value. When the
// value is high, it executes body_stmt and recomputes the conditional
// using cond_group.
fn eval_while(
    w: &iir::While,
    continuous_assignments: &iir::ContinuousAssignments,
    mut env: InterpreterState,
    comp: &iir::Component,
) -> InterpreterResult<InterpreterState> {
    loop {
        if let Some(comb) = &w.cond {
            env = interpret_comb_group(
                Rc::clone(comb),
                continuous_assignments,
                env,
            )?;
        }

        let cond_val = env.get_from_port(&w.port.borrow()).as_bool();

        if let Some(comb) = &w.cond {
            env = finish_comb_group_interpretation(
                &comb.borrow(),
                continuous_assignments,
                env,
            )?;
        }

        if !cond_val {
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
    inv: &Rc<iir::Invoke>,
    continuous_assignments: &iir::ContinuousAssignments,
    env: InterpreterState,
) -> InterpreterResult<InterpreterState> {
    interpret_invoke(inv, continuous_assignments, env)
}

/// Interpret Enable
fn eval_enable(
    e: &iir::Enable,
    continuous_assignments: &iir::ContinuousAssignments,
    mut env: InterpreterState,
) -> InterpreterResult<InterpreterState> {
    env = interpret_group(Rc::clone(&e.group), continuous_assignments, env)?;
    finish_group_interpretation(&e.group.borrow(), continuous_assignments, env)
}

/// Interpret Empty
#[allow(clippy::unnecessary_wraps)]
fn eval_empty(
    _e: &iir::Empty,
    _continuous_assignments: &iir::ContinuousAssignments,
    env: InterpreterState,
    _comp: &iir::Component,
) -> InterpreterResult<InterpreterState> {
    Ok(env)
}
