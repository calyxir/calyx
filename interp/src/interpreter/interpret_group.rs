//! Used for the command line interface.
//! Only interprets a given group in a given component

use crate::environment::Environment;

use crate::utils::OutputValueRef;
use crate::values::{OutputValue, ReadableValue, TimeLockedValue, Value};
use calyx::{
    errors::FutilResult,
    ir::{self, RRC},
};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
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
#[derive(Clone, Debug)]
struct WorkingEnvironment {
    pub backing_env: Environment,
    pub working_env: PortOutputValMap,
}

impl From<Environment> for WorkingEnvironment {
    fn from(input: Environment) -> Self {
        Self {
            working_env: PortOutputValMap::default(),
            backing_env: input,
        }
    }
}

impl WorkingEnvironment {
    fn get(&self, port: &ir::Port) -> OutputValueRef {
        let working_val = self.working_env.get(&(port as ConstPort));
        match working_val {
            Some(v) => v.into(),
            None => self.backing_env.get_from_port(port).into(),
        }
    }

    fn entry(
        &mut self,
        port: &ir::Port,
    ) -> std::collections::hash_map::Entry<ConstPort, OutputValue> {
        self.working_env.entry(port as ConstPort)
    }

    fn update_val(&mut self, port: &ir::Port, value: OutputValue) {
        self.working_env.insert(port as ConstPort, value);
    }

    fn get_as_val(&self, port: &ir::Port) -> &Value {
        match self.get(port) {
            OutputValueRef::ImmediateValue(iv) => iv.get_val(),
            OutputValueRef::LockedValue(tlv) => tlv.get_val(),
            OutputValueRef::PulseValue(pv) => pv.get_val(),
        }
    }

    fn do_tick(&mut self) -> Vec<ConstPort> {
        self.backing_env.clk += 1;

        let mut new_vals = vec![];
        let mut w_env = std::mem::take(&mut self.working_env);

        self.working_env = w_env
            .drain()
            .filter_map(|(port, val)| match val {
                OutputValue::ImmediateValue(iv) => {
                    self.backing_env.insert(port, iv);
                    None
                }
                out @ OutputValue::PulseValue(_)
                | out @ OutputValue::LockedValue(_) => {
                    let old_val = out.get_val().clone();

                    match out.do_tick() {
                        OutputValue::ImmediateValue(iv) => {
                            if iv != old_val {
                                new_vals.push(port)
                            }
                            self.backing_env.insert(port, iv);
                            None
                        }
                        v @ OutputValue::LockedValue(_) => Some((port, v)),
                        OutputValue::PulseValue(pv) => {
                            if *pv.get_val() != old_val {
                                new_vals.push(port);
                            }
                            Some((port, pv.into()))
                        }
                    }
                }
            })
            .collect();
        new_vals
    }

    fn collapse_env(mut self, panic_on_invalid_val: bool) -> Environment {
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
}

fn get_done_port(group: &ir::Group) -> RRC<ir::Port> {
    group.get(&"done")
}

// XXX(Alex): Maybe rename to `eval_is_done`?
fn is_signal_high(done: OutputValueRef) -> bool {
    match done {
        OutputValueRef::ImmediateValue(v) => v.as_u64() == 1,
        OutputValueRef::LockedValue(_) => false,
        OutputValueRef::PulseValue(v) => v.get_val().as_u64() == 1,
    }
}

pub fn interp_assignments<'a, I: Iterator<Item = &'a ir::Assignment>>(
    env: Environment,
    done_signal: &ir::Port,
    assigns: I,
) -> FutilResult<Environment> {
    let assigns = assigns.collect::<Vec<_>>();
    let mut working_env: WorkingEnvironment = env.into();

    let cells = get_cells(assigns.iter().copied());

    let mut val_changed_flag = false;

    while !is_signal_high(working_env.get(done_signal)) || val_changed_flag {
        val_changed_flag = false;

        // do all assigns
        // run all prims
        // if no change, commit value updates

        let mut updates_list = vec![];
        for assignment in &assigns {
            if eval_guard(&assignment.guard, &working_env) {
                let old_val = working_env.get(&assignment.dst.borrow());
                let new_val = working_env.get_as_val(&assignment.src.borrow());

                // no need to make updates if the value has not changed
                if old_val != new_val.into() {
                    val_changed_flag = true;
                    updates_list.push(Rc::clone(&assignment.dst));

                    // Using a TLV to simulate updates happening after reads
                    // Note: this is a hack and should be removed pending
                    // changes in the environment structures
                    // see: https://github.com/cucapra/calyx/issues/549

                    let tmp_old = match old_val.clone_referenced() {
                        OutputValue::ImmediateValue(iv) => Some(iv),
                        OutputValue::LockedValue(tlv) => tlv.old_value,
                        OutputValue::PulseValue(pv) => Some(pv.take_val()),
                    };

                    let new_val = OutputValue::LockedValue(
                        TimeLockedValue::new(new_val.clone(), 0, tmp_old),
                    );

                    let port = &assignment.dst.borrow();

                    working_env.update_val(&port, new_val);
                }
            }
        }

        // Remove the placeholder TLVs
        for port in updates_list {
            if let Entry::Occupied(entry) = working_env.entry(&port.borrow()) {
                let mut_ref = entry.into_mut();
                let v = std::mem::take(mut_ref);

                *mut_ref = if v.is_tlv() {
                    v.unwrap_tlv().unlock().into()
                } else {
                    // this branch should be impossible since the list of
                    // ports we're iterating over are only those w/ updates
                    unreachable!()
                }
            }
        }

        let changed = eval_prims(&mut working_env, cells.iter(), false);
        if changed {
            val_changed_flag = true;
        }

        if !is_signal_high(working_env.get(done_signal)) && !val_changed_flag {
            working_env.do_tick();
            for cell in cells.iter() {
                if let Some(x) = working_env
                    .backing_env
                    .cell_prim_map
                    .get_mut(&(&cell.borrow() as &ir::Cell as *const ir::Cell))
                {
                    x.commit_updates()
                }
            }
        }
    }

    Ok(working_env.collapse_env(false))
}

pub fn interp_cont(
    continuous_assignments: &[ir::Assignment],
    mut env: Environment,
    comp: &ir::Component,
) -> FutilResult<Environment> {
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
    env: Environment,
) -> FutilResult<Environment> {
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
    env: Environment,
) -> FutilResult<Environment> {
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
    let mut prim_map = std::mem::take(&mut env.backing_env.cell_prim_map);

    let mut update_list: Vec<(RRC<ir::Port>, OutputValue)> = vec![];

    for cell in exec_list {
        let inputs = get_inputs(&env, &cell.borrow());

        let executable =
            prim_map.get_mut(&(&cell.borrow() as &ir::Cell as *const ir::Cell));

        if let Some(prim) = executable {
            let new_vals = if reset_flag {
                prim.clear_update_buffer();
                prim.reset(&inputs)
            } else {
                let done_val = if prim.is_comb() {
                    None
                } else {
                    Some(env.get_as_val(&(cell.borrow().get("done").borrow())))
                };
                prim.exec_mut(&inputs, done_val)
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

    env.backing_env.cell_prim_map = prim_map;

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
    mut env: Environment,
    done_signal: &ir::Port,
    assigns: I,
) -> FutilResult<Environment> {
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
    iter.filter_map(|assign| {
        match &assign.dst.borrow().parent {
            ir::PortParent::Cell(c) => {
                match &c.upgrade().borrow().prototype {
                    ir::CellType::Primitive { .. }
                    | ir::CellType::Constant { .. } => Some(c.upgrade()),
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
