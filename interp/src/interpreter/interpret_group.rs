//! Used for the command line interface.
//! Only interprets a given group in a given component

use crate::{environment::InterpreterState, utils::AsRaw};

use super::utils::{get_dest_cells, get_done_port, ConstPort};
use crate::values::Value;
use calyx::ir::{self, RRC};

use super::steppers::AssignmentInterpreter;
use crate::errors::InterpreterResult;

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

use std::rc::Rc;

/// Interprets the given set of continuous assigments and returns a result
/// containing the environment. Note: this is only appropriate to run if the
/// component does not contain groups and indicates doneness via the component's
/// done signal.
///
/// Prior to evaluation the interpreter sets the value of go to high and it
/// returns it to low after execution concludes
pub fn interp_cont<'outer>(
    continuous_assignments: &[ir::Assignment],
    mut env: InterpreterState<'outer>,
    comp: &ir::Component,
) -> InterpreterResult<InterpreterState<'outer>> {
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
    let done_prt_ref = &done_port.borrow() as &ir::Port as *const ir::Port;

    let mut assign_interp = AssignmentInterpreter::new(
        env,
        done_prt_ref,
        (std::iter::empty(), continuous_assignments.iter()),
    );
    assign_interp.run()?;

    let mut res = assign_interp.deconstruct();

    res.insert(
        &go_port.borrow() as &ir::Port as ConstPort,
        Value::bit_low(),
    );

    // required because of lifetime shennanigans
    let final_env = finish_interpretation(
        res,
        &done_port.borrow() as &ir::Port as ConstPort,
        continuous_assignments.iter(),
    );
    final_env
}

/// Evaluates a group, given an environment.
pub fn interpret_group<'outer>(
    group: &ir::Group,
    // TODO (griffin): Use these during interpretation
    continuous_assignments: &[ir::Assignment],
    env: InterpreterState<'outer>,
) -> InterpreterResult<InterpreterState<'outer>> {
    let grp_done = get_done_port(group);
    let grp_done_ref: &ir::Port = &grp_done.borrow();

    let interp = AssignmentInterpreter::new(
        env,
        grp_done_ref,
        (group.assignments.iter(), continuous_assignments.iter()),
    );

    interp.run_and_deconstruct()
}

pub fn finish_group_interpretation<'outer>(
    group: &ir::Group,
    continuous_assignments: &[ir::Assignment],
    env: InterpreterState<'outer>,
) -> InterpreterResult<InterpreterState<'outer>> {
    let grp_done = get_done_port(group);
    let grp_done_ref: &ir::Port = &grp_done.borrow();

    finish_interpretation(
        env,
        grp_done_ref,
        group
            .assignments
            .iter()
            .chain(continuous_assignments.iter()),
    )
}

/// Evaluates the primitives corresponding to the given iterator of cells, based
/// on the current environment. Returns a set of assignments that may change
/// based on the updates to primitive values.
///
/// Note: this function could be written with only one lifetime, but it is worth
/// noting that the returned assignments refs are tied to the dependency map and
/// thus to the assignments it is referencing meanwhile the lifetime on the
/// given cell RRCs is unrelated and largely irrelevant as the prim_map is keyed
/// off of port raw pointers whose lifetime is uncoupled from the cells.
pub(crate) fn eval_prims<'a, 'b, I: Iterator<Item = &'b RRC<ir::Cell>>>(
    env: &mut InterpreterState,
    exec_list: I,
    reset_flag: bool, // reset vals or execute normally
) -> bool {
    let mut val_changed = false;
    // split mutability
    // TODO: change approach based on new env, once ready
    let ref_clone = env.cell_map.clone(); // RC clone
    let mut prim_map = ref_clone.borrow_mut();

    let mut update_list: Vec<(RRC<ir::Port>, Value)> = vec![];

    for cell in exec_list {
        let inputs = get_inputs(env, &cell.borrow());

        let executable = prim_map.get_mut(&cell.as_raw());

        if let Some(prim) = executable {
            let new_vals = if reset_flag {
                prim.reset(&inputs)
            } else {
                prim.execute(&inputs)
            };

            for (port, val) in new_vals {
                let port_ref = cell.borrow().find(port).unwrap();

                let current_val = env.get_from_port(&port_ref.borrow());

                if *current_val != val {
                    val_changed = true;
                    // defer value update until after all executions
                    update_list.push((Rc::clone(&port_ref), val));
                }
            }
        }
    }

    for (port, val) in update_list {
        env.insert(port, val);
    }

    val_changed
}

fn get_inputs<'a>(
    env: &'a InterpreterState,
    cell: &ir::Cell,
) -> Vec<(ir::Id, &'a Value)> {
    cell.ports
        .iter()
        .filter_map(|p| {
            let p_ref: &ir::Port = &p.borrow();
            match &p_ref.direction {
                ir::Direction::Input => {
                    Some((p_ref.name.clone(), env.get_from_port(p_ref)))
                }
                _ => None,
            }
        })
        .collect()
}

/// Concludes interpretation to a group, effectively setting the go signal low
/// for a given group. This function updates the values in the environment
/// accordingly using zero as a placeholder for values that are undefined
pub(crate) fn finish_interpretation<
    'a,
    I: Iterator<Item = &'a ir::Assignment>,
    P: AsRaw<ir::Port>,
>(
    mut env: InterpreterState,
    done_signal: P,
    assigns: I,
) -> InterpreterResult<InterpreterState> {
    // replace port values for all the assignments
    let assigns = assigns.collect::<Vec<_>>();

    for &ir::Assignment { dst, .. } in &assigns {
        env.insert(
            &dst.borrow() as &ir::Port as ConstPort,
            Value::zeroes(dst.borrow().width as usize),
        );
    }

    let cells = get_dest_cells(assigns.iter().copied());

    env.insert(done_signal.as_raw(), Value::bit_low());
    eval_prims(&mut env, cells.iter(), true);

    Ok(env)
}
