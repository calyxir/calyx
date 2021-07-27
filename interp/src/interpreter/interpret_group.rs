//! Used for the command line interface.
//! Only interprets a given group in a given component

use crate::environment::InterpreterState;

use crate::utils::{get_const_from_rrc, OutputValueRef};
use crate::values::{OutputValue, ReadableValue, Value};
use calyx::{
    errors::FutilResult,
    ir::{self, RRC},
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

type ConstPort = *const ir::Port;

/// A wrapper for a map assigning OutputValues to each port. Used in the working
/// environment to track values that are not of type Value which is used in the
/// environment.
// TODO (griffin): Update environment definition to allow for things of type
//                 OutputValue?
type PortOutputValMap = HashMap<ConstPort, OutputValue>;

/// A wrapper struct to keep the passed environment and a map tracking the
/// updates made in the current environment. It is only really needed because
/// the environment maps to values of type Value, but during group
/// interpretation, ports need to be mapped to values of type OutputValue
// TODO (griffin): Update / remove pending changes to environment definition
struct WorkingEnvironment {
    //InterpreterState has a pv_map which is a Smoosher<*const ir::Port, Value>
    pub backing_env: InterpreterState,
    pub working_env: PortOutputValMap, // HashMap<*const ir::Port, OutputValue>
}

impl From<InterpreterState> for WorkingEnvironment {
    fn from(input: InterpreterState) -> Self {
        Self {
            backing_env: input,
            working_env: PortOutputValMap::default(),
        }
    }
}

impl WorkingEnvironment {
    fn get_const(&self, port: *const ir::Port) -> OutputValueRef {
        let working_val = self.working_env.get(&port);
        match working_val {
            Some(v) => v.into(),
            None => self.backing_env.get_from_const_port(port).into(),
        }
    }
    /// Attempts to first get value from the working_env (PortOutputValMap)
    /// If doesn't exist, gets from backing_env (InterpreterState)
    fn get(&self, port: &ir::Port) -> OutputValueRef {
        self.get_const(port as *const ir::Port)
    }

    fn update_val_const_port(
        &mut self,
        port: *const ir::Port,
        value: OutputValue,
    ) {
        self.working_env.insert(port, value);
    }

    fn update_val(&mut self, port: &ir::Port, value: OutputValue) {
        self.update_val_const_port(port as *const ir::Port, value);
    }

    fn get_as_val_const(&self, port: *const ir::Port) -> &Value {
        match self.get_const(port) {
            OutputValueRef::ImmediateValue(iv) => iv.get_val(),
            OutputValueRef::LockedValue(tlv) => tlv.get_val(),
            OutputValueRef::PulseValue(pv) => pv.get_val(),
        }
    }

    fn get_as_val(&self, port: &ir::Port) -> &Value {
        self.get_as_val_const(port as *const ir::Port)
    }

    //for use w/ smoosher: maybe add a new scope onto backing_env for the tick?
    //this is not used now w/ primitives having do_tick(). they are ticked in interp_assignments()
    fn do_tick(&mut self) {
        self.backing_env.clk += 1;
    }

    fn collapse_env(mut self, panic_on_invalid_val: bool) -> InterpreterState {
        let working_env = self.working_env;

        for (port, v) in working_env {
            match v {
                OutputValue::ImmediateValue(iv) => {
                    self.backing_env.insert(port, iv)
                }
                OutputValue::LockedValue(tlv) => {
                    if tlv.unlockable() {
                        let iv = tlv.unlock();
                        self.backing_env.insert(port, iv);
                    } else if panic_on_invalid_val {
                        panic!("Group is done with an invalid value?")
                    } else if let Some(old) = tlv.old_value {
                        self.backing_env.insert(port, old)
                    }
                }
                OutputValue::PulseValue(v) => {
                    self.backing_env.insert(port, v.take_val())
                }
            }
        }
        self.backing_env
    }

    // For debugging purpose
    /*fn _dump_state(&self, cell: &ir::Cell) {
        println!("{} on cycle {}: ", cell.name(), self.backing_env.clk);
        for p in &cell.ports {
            let p_ref: &ir::Port = &p.borrow();
            println!("  {} : {}", p_ref.name, self.get_as_val(p_ref).as_u64());
        }
        match self
            .backing_env
            .cell_prim_map
            .borrow()
            .get(&(cell as *const ir::Cell))
            .unwrap()
        {
            Primitive::StdReg(ref reg) => {
                println!("  internal state: {}", reg.data[0])
            }
            Primitive::StdMemD1(ref mem) => {
                println!("  memval : {}", mem.data[0])
            }
            _ => {}
        }
    }*/
}

fn get_done_port(group: &ir::Group) -> RRC<ir::Port> {
    group.get(&"done")
}

fn is_signal_high(done: OutputValueRef) -> bool {
    match done {
        OutputValueRef::ImmediateValue(v) => v.as_u64() == 1,
        OutputValueRef::LockedValue(_) => false,
        OutputValueRef::PulseValue(v) => v.get_val().as_u64() == 1,
    }
}

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

    let cells = get_cells(assigns.iter().copied());

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
            if eval_guard(&assignment.guard, &working_env) {
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
                Value::from(0, old_val_width).unwrap().into();
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

        let changed = eval_prims(&mut working_env, cells.iter(), false);
        if changed {
            val_changed_flag = true;
        }

        //if done signal is low and we haven't yet changed anything, means primitives are done,
        //time to evaluate sequential components
        if !is_signal_high(working_env.get(done_signal)) && !val_changed_flag {
            let mut update_list: Vec<(RRC<ir::Port>, OutputValue)> = vec![];

            //no need to do zero-assign check cuz this is run just once (?)
            for cell in cells.iter() {
                if let Some(x) =
                    working_env.backing_env.cell_prim_map.borrow_mut().get_mut(
                        &(&cell.borrow() as &ir::Cell as *const ir::Cell),
                    )
                {
                    let new_vals = x.do_tick();
                    for (port, val) in new_vals {
                        let port_ref = cell.borrow().find(port).unwrap();

                        update_list.push((Rc::clone(&port_ref), val));
                    }

                    //call do_tick on each cell's primitive, and then
                    //write the updates as done in eval_prims
                    //specifically line 419, 448, 455 (create update list, push to it, env.update_val)
                    //then call working_env.do_tick(), so everything is written to backing_env for us :=)
                    //no need to worry about done values anymore; the done values will be written
                    //to env, and primitives will remember their previous [done] state
                    //no need to treat [done] special anymore.
                }
            }
            //put everything from update list into working_env, then
            //working_env will put the IMM values (all values) into
            //backing_env
            for (port, val) in update_list {
                working_env.update_val(&port.borrow(), val);
            }
            working_env.do_tick();

            //after this if statement runs ONCE, end of a cycle. Should only run once!
            //but prims above can run as much as they want before they stabilize
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

/// Evaluates the primitives corresponding to the given iterator of cells, based
/// on the current environment. Returns a set of assignments that may change
/// based on the updates to primitive values.
///
/// Note: this function could be written with only one lifetime, but it is worth
/// noting that the returned assignments refs are tied to the dependency map and
/// thus to the assignments it is referencing meanwhile the lifetime on the
/// given cell RRCs is unrelated and largely irrelevant as the prim_map is keyed
/// off of port raw pointers whose lifetime is uncoupled from the cells.
fn eval_prims<'a, 'b, I: Iterator<Item = &'b RRC<ir::Cell>>>(
    env: &mut WorkingEnvironment,
    exec_list: I,
    reset_flag: bool, // reset vals or execute normally
) -> bool {
    let mut val_changed = false;
    // split mutability
    // TODO: change approach based on new env, once ready
    let ref_clone = env.backing_env.cell_prim_map.clone(); // RC clone
    let mut prim_map = ref_clone.borrow_mut();

    let mut update_list: Vec<(RRC<ir::Port>, OutputValue)> = vec![];

    for cell in exec_list {
        let inputs = get_inputs(&env, &cell.borrow());

        let executable = prim_map.get_mut(&get_const_from_rrc(&cell));

        if let Some(prim) = executable {
            //note: w/ new do_tick() interface, no need for this [done] stuff
            let new_vals = if reset_flag {
                //note: need to clear update buffer in reset
                prim.reset(&inputs)
            } else {
                prim.execute(&inputs)
            };

            for (port, val) in new_vals {
                let port_ref = cell.borrow().find(port).unwrap();

                let current_val = env.get(&port_ref.borrow());

                if current_val != (&val).into() {
                    val_changed = true;
                    // defer value update until after all executions
                    update_list.push((Rc::clone(&port_ref), val));
                }
            }
        }
    }

    for (port, val) in update_list {
        env.update_val(&port.borrow(), val);
    }

    val_changed
}

fn get_inputs<'a>(
    env: &'a WorkingEnvironment,
    cell: &ir::Cell,
) -> Vec<(ir::Id, &'a Value)> {
    cell.ports
        .iter()
        .filter_map(|p| {
            let p_ref: &ir::Port = &p.borrow();
            match &p_ref.direction {
                ir::Direction::Input => {
                    Some((p_ref.name.clone(), env.get_as_val(p_ref)))
                }
                _ => None,
            }
        })
        .collect()
}

fn eval_guard(guard: &ir::Guard, env: &WorkingEnvironment) -> bool {
    match guard {
        ir::Guard::Or(g1, g2) => eval_guard(g1, env) || eval_guard(g2, env),
        ir::Guard::And(g1, g2) => eval_guard(g1, env) && eval_guard(g2, env),
        ir::Guard::Not(g) => !eval_guard(g, &env),
        ir::Guard::Eq(g1, g2) => {
            env.get_as_val(&g1.borrow()) == env.get_as_val(&g2.borrow())
        }
        ir::Guard::Neq(g1, g2) => {
            env.get_as_val(&g1.borrow()) != env.get_as_val(&g2.borrow())
        }
        ir::Guard::Gt(g1, g2) => {
            env.get_as_val(&g1.borrow()) > env.get_as_val(&g2.borrow())
        }
        ir::Guard::Lt(g1, g2) => {
            env.get_as_val(&g1.borrow()) < env.get_as_val(&g2.borrow())
        }
        ir::Guard::Geq(g1, g2) => {
            env.get_as_val(&g1.borrow()) >= env.get_as_val(&g2.borrow())
        }
        ir::Guard::Leq(g1, g2) => {
            env.get_as_val(&g1.borrow()) <= env.get_as_val(&g2.borrow())
        }
        ir::Guard::Port(p) => {
            let val = env.get_as_val(&p.borrow());
            if val.vec.len() != 1 {
                panic!(
                    "Evaluating the truth value of a wire '{:?}' that is not one bit", p.borrow().canonical()
                )
            } else {
                val.as_u64() == 1
            }
        }
        ir::Guard::True => true,
    }
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

    let cells = get_cells(assigns.iter().copied());

    env.insert(done_signal as ConstPort, Value::bit_low());
    let mut working_env: WorkingEnvironment = env.into();
    eval_prims(&mut working_env, cells.iter(), true);

    Ok(working_env.collapse_env(false))
}

fn get_cells<'a, I>(iter: I) -> Vec<RRC<ir::Cell>>
where
    I: Iterator<Item = &'a ir::Assignment>,
{
    let mut assign_set: HashSet<*const ir::Cell> = HashSet::new();
    iter.filter_map(|assign| {
        match &assign.dst.borrow().parent {
            ir::PortParent::Cell(c) => {
                match &c.upgrade().borrow().prototype {
                    ir::CellType::Primitive { .. }
                    | ir::CellType::Constant { .. } => {
                        let const_cell: *const ir::Cell = c.upgrade().as_ptr();
                        if assign_set.contains(&const_cell) {
                            None //b/c we don't want duplicates
                        } else {
                            assign_set.insert(const_cell);
                            Some(c.upgrade())
                        }
                    }
                    ir::CellType::Component { .. } => {
                        // TODO (griffin): We'll need to handle this case at some point
                        todo!()
                    }
                    ir::CellType::ThisComponent => None,
                }
            }
            ir::PortParent::Group(_) => None,
        }
    })
    .collect()
}
