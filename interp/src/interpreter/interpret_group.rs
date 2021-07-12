//! Used for the command line interface.
//! Only interprets a given group in a given component

use crate::environment::InterpreterState;

use super::simulation_utils::{
    get_done_port, get_dst_cells, is_signal_high, ConstPort,
};
use super::working_environment::WorkingEnvironment;
use crate::primitives::Primitive;
use crate::utils::{get_const_from_rrc, OutputValueRef};
use crate::values::{OutputValue, ReadableValue, Value};
use calyx::{
    errors::FutilResult,
    ir::{self, RRC},
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::steppers::AssignmentInterpreter;

// /// An internal method that does the main work of interpreting a set of
// /// assignments. It takes the assigments as an interator as continguity of
// /// memory is not a requirement and importantly, the function must also be
// /// provided with a port which will be treated as the revelant done signal for
// /// the execution
// fn interp_assignments<'a, I: Iterator<Item = &'a ir::Assignment>>(
//     env: InterpreterState,
//     done_signal: &ir::Port,
//     assigns: I,
// ) -> FutilResult<InterpreterState> {
// }

/// Interprets the given set of continuous assigments and returns a result
/// containing the environment. Note: this is only appropriate to run if the
/// component does not contain groups and indicates doneness via the component's
/// done signal.
///
/// Prior to evaluation the interpreter sets the value of go to high and it
/// returns it to low after execution concludes
pub fn interp_cont(
    continuous_assignments: &[ir::Assignment],
    mut env: InterpreterState,
    comp: &ir::Component,
) -> FutilResult<InterpreterState> {
    let comp_sig = comp.signature.borrow();

    let go_port = comp_sig
        .ports
        .iter()
        .find(|x| x.borrow().name == "go")
        .unwrap();

    let done_port = comp_sig
        .ports
        .iter()
        .find(|x| x.borrow().name == "done")
        .unwrap();

    env.insert(
        &go_port.borrow() as &ir::Port as ConstPort,
        Value::bit_high(),
    );
    let done_prt_ref = &done_port.borrow();

    let mut assign_interp = AssignmentInterpreter::new(
        env,
        done_prt_ref,
        continuous_assignments.iter(),
    );
    assign_interp.run_group();

    let mut res = assign_interp.deconstruct_no_check();

    res.insert(
        &go_port.borrow() as &ir::Port as ConstPort,
        Value::bit_low(),
    );

    // required because of lifetime shennanigans
    let final_env = AssignmentInterpreter::finish_interpretation(
        res,
        &done_port.borrow(),
        continuous_assignments.iter(),
    );

    Ok(final_env)
}

/// Evaluates a group, given an environment.
pub fn interpret_group(
    group: &ir::Group,
    // TODO (griffin): Use these during interpretation
    continuous_assignments: &[ir::Assignment],
    env: InterpreterState,
) -> FutilResult<InterpreterState> {
    let grp_done = get_done_port(&group);
    let grp_done_ref: &ir::Port = &grp_done.borrow();

    let interp = AssignmentInterpreter::new(
        env,
        grp_done_ref,
        group
            .assignments
            .iter()
            .chain(continuous_assignments.iter()),
    );

    Ok(interp.run_and_deconstruct())
}

pub fn finish_group_interpretation(
    group: &ir::Group,
    // TODO (griffin): Use these during interpretation
    continuous_assignments: &[ir::Assignment],
    env: InterpreterState,
) -> FutilResult<InterpreterState> {
    let grp_done = get_done_port(&group);
    let grp_done_ref: &ir::Port = &grp_done.borrow();

    Ok(AssignmentInterpreter::finish_interpretation(
        env,
        grp_done_ref,
        group
            .assignments
            .iter()
            .chain(continuous_assignments.iter()),
    ))
}
