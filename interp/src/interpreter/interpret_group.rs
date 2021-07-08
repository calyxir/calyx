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

/// An internal method that does the main work of interpreting a set of
/// assignments. It takes the assigments as an interator as continguity of
/// memory is not a requirement and importantly, the function must also be
/// provided with a port which will be treated as the revelant done signal for
/// the execution
fn interp_assignments<'a, I: Iterator<Item = &'a ir::Assignment>>(
    env: InterpreterState,
    done_signal: &ir::Port,
    assigns: I,
) -> FutilResult<InterpreterState> {
    let assigns = assigns.collect_vec();
    let mut working_env: WorkingEnvironment = env.into(); //env as backing_env, fresh slate as working_env

    let cells = get_dst_cells(assigns.iter().copied());

    //another issue w/ using smoosher: say we are in tick X. If the guard fails
    //for a given port N, and that guard has failed since tick X, would we know
    //to assign N a zero? The first tick has to be done seperately
    //so that all ports in [assigns] are put in the bottom scope of the Smoosher
    //(failed guards go in as zeroes)
    //and the we can trust it to catch unassigned port sin higher scopes using
    //perhaps smoosher.tail_to_hm() - smoosher.top(). But still the issue of output
    //values ? No, we don't intend to change the WorkingEnvironment struct, just this
    //possible_ports stuff
    let possible_ports: HashSet<*const ir::Port> =
        assigns.iter().map(|a| get_const_from_rrc(&a.dst)).collect();
    let mut val_changed_flag = false;

    while !is_signal_high(working_env.get(done_signal)) || val_changed_flag {
        //helps us tell if there are multiple assignments to same port >:0
        let mut assigned_ports: HashSet<*const ir::Port> = HashSet::new();
        val_changed_flag = false;

        // do all assigns
        // run all prims
        // if no change, commit value updates

        let mut updates_list = vec![];
        // compute all updates from the assignments
        for assignment in &assigns {
            // if assignment.dst.borrow().name == "done"
            // println!("{:?}", assignment.);
            if working_env.eval_guard(&assignment.guard) {
                //if we change to smoosher, we need to add functionality that
                //still prevents multiple drivers to same port, like below
                //Perhaps use Smoosher's diff_other func?

                //first check nothing has been assigned to this destination yet
                if assigned_ports.contains(&get_const_from_rrc(&assignment.dst))
                {
                    let dst = assignment.dst.borrow();
                    panic!(
                        "[interpret_group]: multiple assignments to one port: {}.{}", dst.get_parent_name(), dst.name
                    );
                }
                //now add to the HS, because we are assigning
                //regardless of whether value has changed this is still a
                //value driving the port
                assigned_ports.insert(get_const_from_rrc(&assignment.dst));
                //ok now proceed
                //the below (get) attempts to get from working_env HM first, then
                //backing_env Smoosher. What does it mean for the value to be in HM?
                //That it's a locked value?
                let old_val = working_env.get(&assignment.dst.borrow());
                let new_val_ref =
                    working_env.get_as_val(&assignment.src.borrow());

                // no need to make updates if the value has not changed
                let port = assignment.dst.clone(); // Rc clone
                let new_val: OutputValue = new_val_ref.clone().into();

                if old_val != new_val_ref.into() {
                    updates_list.push((port, new_val)); //no point in rewriting same value to this list

                    val_changed_flag = true;
                }
            }
        }

        //now assign rest to 0
        //first get all that need to be 0
        for port in &possible_ports - &assigned_ports {
            //need to set to zero, because unassigned
            //ok now proceed

            //need to find appropriate-sized 0, so just read
            //width of old_val

            let old_val = working_env.get_as_val_const(port);
            let old_val_width = old_val.width(); //&assignment.dst.borrow().width()
            let new_val: OutputValue =
                Value::try_from_init(0, old_val_width).unwrap().into();
            //updates_list.push((port, new_val));

            //how to avoid infinite loop?
            //if old_val is imm value and zero, then that's
            //when val_changed_flag is false, else true.
            if old_val.as_u64() != 0 {
                val_changed_flag = true;
            }

            //update directly
            working_env.update_val_const_port(port, new_val);
        }

        // perform all the updates
        for (port, value) in updates_list {
            working_env.update_val(&port.borrow(), value);
        }

        let changed = working_env.eval_prims(cells.iter(), false);
        if changed {
            val_changed_flag = true;
        }

        //if done signal is low and we haven't yet changed anything, means primitives are done,
        //time to evaluate sequential components
        if !is_signal_high(working_env.get(done_signal)) && !val_changed_flag {
            working_env.do_tick();
            for cell in cells.iter() {
                if let Some(x) =
                    working_env.backing_env.cell_prim_map.borrow_mut().get_mut(
                        &(&cell.borrow() as &ir::Cell as *const ir::Cell),
                    )
                {
                    x.commit_updates()
                }
            }
        }
    }

    Ok(working_env.collapse_env(false))
}

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

    let mut res = interp_assignments(
        env,
        &done_port.borrow(),
        continuous_assignments.iter(),
    )?;

    res.insert(
        &go_port.borrow() as &ir::Port as ConstPort,
        Value::bit_low(),
    );

    // required because of lifetime shennanigans
    let final_env = finish_interpretation(
        res,
        &done_port.borrow(),
        continuous_assignments.iter(),
    );

    final_env
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
    interp_assignments(
        env,
        grp_done_ref,
        group
            .assignments
            .iter()
            .chain(continuous_assignments.iter()),
    )
}

pub fn finish_group_interpretation(
    group: &ir::Group,
    // TODO (griffin): Use these during interpretation
    continuous_assignments: &[ir::Assignment],
    env: InterpreterState,
) -> FutilResult<InterpreterState> {
    let grp_done = get_done_port(&group);
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

/// Concludes interpretation to a group, effectively setting the go signal low
/// for a given group. This function updates the values in the environment
/// accordingly using zero as a placeholder for values that are undefined
fn finish_interpretation<'a, I: Iterator<Item = &'a ir::Assignment>>(
    mut env: InterpreterState,
    done_signal: &ir::Port,
    assigns: I,
) -> FutilResult<InterpreterState> {
    // replace port values for all the assignments
    let assigns = assigns.collect::<Vec<_>>();

    for &ir::Assignment { dst, .. } in &assigns {
        env.insert(
            &dst.borrow() as &ir::Port as ConstPort,
            Value::zeroes(dst.borrow().width as usize),
        );
    }

    let cells = get_dst_cells(assigns.iter().copied());

    env.insert(done_signal as ConstPort, Value::bit_low());
    let mut working_env: WorkingEnvironment = env.into();
    working_env.eval_prims(cells.iter(), true);

    Ok(working_env.collapse_env(false))
}
