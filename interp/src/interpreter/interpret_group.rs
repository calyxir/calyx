//! Used for the command line interface.
//! Only interprets a given group in a given component

use crate::environment::Environment;

use crate::utils::{AssignmentRef, CellRef, OutputValueRef};
use crate::values::{OutputValue, ReadableValue, TimeLockedValue, Value};
use calyx::{
    errors::FutilResult,
    ir::{self, RRC},
};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::iter;
use std::rc::Rc;

type ConstPort = *const ir::Port;

/// A wrapper over a hashmap keyed by raw pointers. It contains a mapping from
/// output (or inout) ports to a set of assignments which read from/depend on
/// the given port. This is used to avoid re-running assignments when relevant
/// values have not changed.
#[derive(Debug, Clone, Default)]
struct DependencyMap<'a> {
    map: HashMap<ConstPort, HashSet<AssignmentRef<'a>>>,
}

impl<'a, I: Iterator<Item = &'a ir::Assignment>> From<I> for DependencyMap<'a> {
    fn from(iter: I) -> Self {
        let mut map = DependencyMap::default();
        map.populate_map(iter);
        map
    }
}

impl<'a> DependencyMap<'a> {
    fn populate_map<I: Iterator<Item = &'a ir::Assignment>>(
        &mut self,
        iter: I,
    ) {
        for assignment in iter {
            let ports = assignment
                .guard
                .all_ports()
                .into_iter()
                .chain(iter::once(assignment.src.clone()))
                .chain(iter::once(assignment.dst.clone()));
            for port in ports {
                if match &port.borrow().direction {
                    ir::Direction::Input => false,
                    ir::Direction::Output | ir::Direction::Inout => true,
                } {
                    self.map
                        .entry(&port.borrow() as &ir::Port as ConstPort)
                        .or_default()
                        .insert(assignment.into());
                }
            }
        }
    }

    fn get(&self, port: &ir::Port) -> Option<&HashSet<AssignmentRef<'a>>> {
        self.map.get(&(port as ConstPort))
    }
}
/// An alias for a hashset over Assignments (hashed with a wrapper using
/// identity). Used to track assignments that need to be (re)evaluated
type WorkList<'a> = HashSet<AssignmentRef<'a>>;

/// An alias for a hashset over cells (hashed with a wrapper using identity).
/// Used to track when cells should be (re)evaluated due to changes on one or
/// more of their inputs
type CellList = HashSet<CellRef>;

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

    fn collapse_env(mut self) -> Environment {
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
                    } else {
                        panic!("Group is done with an invalid value?")
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

fn grp_is_done(done: OutputValueRef) -> bool {
    match done {
        OutputValueRef::ImmediateValue(v) => v.as_u64() == 1,
        OutputValueRef::LockedValue(_) => false,
        OutputValueRef::PulseValue(v) => v.get_val().as_u64() == 1,
    }
}

/// Evaluates a group, given an environment.
pub fn interpret_group(
    group: &ir::Group,
    // TODO (griffin): Use these during interpretation
    _continuous_assignments: &[ir::Assignment],
    env: Environment,
) -> FutilResult<Environment> {
    let dependency_map = group.assignments.iter().into();
    let grp_done = get_done_port(&group);
    let mut working_env: WorkingEnvironment = env.into();
    let mut assign_worklist: WorkList =
        group.assignments.iter().map(|x| x.into()).collect();
    let mut comb_cells = CellList::new();
    let mut non_comb_cells = CellList::new();

    while !grp_is_done(working_env.get(&grp_done.borrow()))
        || !comb_cells.is_empty()
        || !assign_worklist.is_empty()
    // Note: May need to remove later
    {
        if !comb_cells.is_empty() {
            let tmp = std::mem::take(&mut comb_cells);

            let new_assigns = eval_prims(
                &mut working_env,
                &dependency_map,
                tmp.iter().map(|x| x.into()),
                false,
            );

            assign_worklist.extend(new_assigns.into_iter())
        } else if !assign_worklist.is_empty() {
            let mut updates_list = vec![];
            let mut new_worklist = WorkList::new();

            // STEP 1 : Evaluate all assignments
            for assignment in assign_worklist.drain() {
                if eval_guard(&assignment.guard, &working_env) {
                    let old_val = working_env.get(&assignment.dst.borrow());
                    let new_val =
                        working_env.get_as_val(&assignment.src.borrow());

                    // no need to make updates if the value has not changed
                    if old_val != new_val.into() {
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

                        let cell = match &port.parent {
                            ir::PortParent::Cell(c) => Some(c.upgrade()),
                            ir::PortParent::Group(_) => None,
                        };

                        if let Some(cell) = cell {
                            if working_env
                                .backing_env
                                .cell_is_comb(&cell.borrow())
                            {
                                comb_cells.insert(cell.into());
                            } else {
                                non_comb_cells.insert(cell.into());
                            }
                        }

                        let new_assigments = dependency_map.get(port);

                        if let Some(new_assigments) = new_assigments {
                            new_worklist.extend(new_assigments.iter().cloned());
                        }
                    }
                }
            }

            assign_worklist = new_worklist;

            // Remove the placeholder TLVs
            for port in updates_list {
                if let Entry::Occupied(entry) =
                    working_env.entry(&port.borrow())
                {
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
        } else if !non_comb_cells.is_empty() {
            let tmp = std::mem::take(&mut non_comb_cells);

            let new_assigns = eval_prims(
                &mut working_env,
                &dependency_map,
                tmp.iter().map(|x| x.into()),
                false,
            );
            assign_worklist.extend(new_assigns.into_iter())
        }
        // all are empty
        else {
            let assigns: WorkList = working_env
                .do_tick()
                .into_iter()
                .filter_map(|port| dependency_map.map.get(&port))
                .flatten()
                .cloned()
                .collect();

            assign_worklist = assigns;
        }
    }

    Ok(working_env.collapse_env())
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
    dependency_map: &DependencyMap<'a>,
    exec_list: I,
    reset_flag: bool, // reset vals or execute normally
) -> HashSet<AssignmentRef<'a>> {
    // split mutability
    // TODO: change approach based on new env, once ready
    let mut prim_map = std::mem::take(&mut env.backing_env.cell_prim_map);

    let mut update_list: Vec<(RRC<ir::Port>, OutputValue)> = vec![];
    let mut assign_list: HashSet<AssignmentRef> = HashSet::new();

    for cell in exec_list {
        let inputs = get_inputs(&env, &cell.borrow());

        let executable =
            prim_map.get_mut(&(&cell.borrow() as &ir::Cell as *const ir::Cell));

        if let Some(prim) = executable {
            let new_vals = if reset_flag {
                prim.reset(&inputs)
            } else {
                prim.exec_mut(&inputs)
            };

            for (port, val) in new_vals {
                let port_ref = cell.borrow().find(port).unwrap();

                let current_val = env.get(&port_ref.borrow());

                if current_val != (&val).into() {
                    // defer value update until after all executions
                    update_list.push((Rc::clone(&port_ref), val));

                    let new_assigments = dependency_map.get(&port_ref.borrow());
                    if let Some(assigns) = new_assigments {
                        assign_list.extend(assigns.iter().cloned());
                    }
                }
            }
        }
    }

    for (port, val) in update_list {
        env.update_val(&port.borrow(), val);
    }

    env.backing_env.cell_prim_map = prim_map;
    assign_list
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
pub fn finish_group_interpretation(
    group: &ir::Group,
    _continuous_assignments: &[ir::Assignment],
    mut env: Environment,
) -> FutilResult<Environment> {
    // replace port values for all the assignments

    let done = get_done_port(group);

    let cells: Vec<RRC<ir::Cell>> = group
        .assignments
        .iter()
        .filter_map(|ir::Assignment { dst, .. }| {
            env.insert(
                &dst.borrow() as &ir::Port as ConstPort,
                Value::zeroes(dst.borrow().width as usize),
            );
            match &dst.borrow().parent {
                ir::PortParent::Cell(c) => Some(c.upgrade()),
                ir::PortParent::Group(_) => None,
            }
        })
        .collect();

    env.insert(&done.borrow() as &ir::Port as ConstPort, Value::zeroes(1));

    let mut working_env: WorkingEnvironment = env.into();

    eval_prims(
        &mut working_env,
        &DependencyMap::default(),
        cells.iter(),
        true,
    );

    Ok(working_env.collapse_env())
}
