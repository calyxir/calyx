//! Used for the command line interface.
//! Only interprets a given group in a given component

use crate::environment::Environment;

use crate::utils::{AssignmentRef, CellRef, OutputValueRef};
use crate::values::{OutputValue, TimeLockedValue, Value};
use calyx::{
    errors::{Error, FutilResult},
    ir::{self, CloneName, RRC},
};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::iter;
use std::rc::Rc;

#[allow(unused_imports)]
use crate::primitives::{
    Execute, ExecuteBinary, ExecuteStateful, ExecuteUnary,
};
#[derive(Debug, Clone, Default)]
struct DependencyMap<'a> {
    map: HashMap<*const ir::Port, HashSet<AssignmentRef<'a>>>,
}

impl<'a> DependencyMap<'a> {
    fn from_assignments<I: Iterator<Item = &'a ir::Assignment>>(
        iter: I,
    ) -> DependencyMap<'a> {
        let mut map = DependencyMap::default();
        map.populate_map(iter);
        map
    }

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
                        .entry(&port.borrow() as &ir::Port as *const ir::Port)
                        .or_default()
                        .insert(assignment.into());
                }
            }
        }
    }

    fn get(&self, port: &ir::Port) -> Option<&HashSet<AssignmentRef<'a>>> {
        self.map.get(&(port as *const ir::Port))
    }
}

type WorkList<'a> = HashSet<AssignmentRef<'a>>;
type CellList = HashSet<CellRef>;

type PortOutputValMap = HashMap<*const ir::Port, OutputValue>;

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
        let working_val = self.working_env.get(&(port as *const ir::Port));
        match working_val {
            Some(v) => v.into(),
            None => self.backing_env.get_from_port(port).into(),
        }
    }

    fn entry(
        &mut self,
        port: &ir::Port,
    ) -> std::collections::hash_map::Entry<*const calyx::ir::Port, OutputValue>
    {
        self.working_env.entry(port as *const ir::Port)
    }

    fn update_val(&mut self, port: &ir::Port, value: OutputValue) {
        self.working_env.insert(port as *const ir::Port, value);
    }

    fn get_as_val(&self, port: &ir::Port) -> &Value {
        match self.get(port) {
            OutputValueRef::ImmediateValue(iv) => iv,
            OutputValueRef::LockedValue(tlv) => {
                &tlv.old_value.as_ref().unwrap_or_else(|| {
                    panic!("Attempting to read an invalid value")
                })
            }
        }
    }

    fn do_tick(&mut self) -> Vec<*const ir::Port> {
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
                OutputValue::LockedValue(mut tlv) => {
                    tlv.dec_count();
                    if tlv.unlockable() {
                        let iv = tlv.unlock();
                        if iv != self.backing_env.pv_map[&port] {
                            self.backing_env.insert(port, iv);
                            new_vals.push(port)
                        }
                        None
                    } else {
                        Some((port, tlv.into()))
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
            }
        }
        self.backing_env
    }
}

// possibly #[inline] here later? Compiler probably knows to do that already
fn get_done_port(group: &ir::Group) -> RRC<ir::Port> {
    group.get(&"done")
}

fn grp_is_done(done: OutputValueRef) -> bool {
    match done {
        OutputValueRef::ImmediateValue(v) => v.as_u64() == 1,
        OutputValueRef::LockedValue(_) => false,
    }
}

/// Evaluates a group, given an environment.
pub fn interpret_group(
    group: &ir::Group,
    _continuous_assignments: &[ir::Assignment],
    env: Environment,
) -> FutilResult<Environment> {
    let dependency_map =
        DependencyMap::from_assignments(group.assignments.iter());
    let grp_done = get_done_port(&group);
    let mut working_env: WorkingEnvironment = env.into();
    let mut assign_worklist: WorkList =
        group.assignments.iter().map(|x| x.into()).collect();
    let mut comb_cells = CellList::new();
    let mut non_comb_cells = CellList::new();

    while !grp_is_done(working_env.get(&grp_done.borrow())) {
        if !comb_cells.is_empty() {
            let tmp = std::mem::take(&mut comb_cells);

            let new_assigns = eval_prims(
                &mut working_env,
                &dependency_map,
                tmp.iter().map(|x| x.into()),
            );

            assign_worklist.extend(new_assigns.into_iter())
        } else if !assign_worklist.is_empty() {
            let mut updates_list = vec![];
            let mut exec_list: Vec<RRC<ir::Cell>> = vec![];
            let mut new_worklist = WorkList::new();

            // STEP 1 : Evaluate all assignments
            for assignment in assign_worklist.drain() {
                if eval_guard(&assignment.guard, &working_env) {
                    let old_val = working_env.get(&assignment.dst.borrow());
                    let new_val =
                        working_env.get_as_val(&assignment.src.borrow());

                    if old_val != new_val.into() {
                        updates_list.push(Rc::clone(&assignment.dst));

                        let tmp_old = match old_val.clone_referenced() {
                            OutputValue::ImmediateValue(iv) => Some(iv),
                            OutputValue::LockedValue(tlv) => tlv.old_value,
                        };

                        let new_val = OutputValue::LockedValue(
                            TimeLockedValue::new(new_val.clone(), 0, tmp_old),
                        );

                        // STEP 2 : Update values and determine new worklist and exec_list

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

            // STEP 2.5 : Remove the placeholder TLVs
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
                // check if the current val of id matches the new update
                // if yes, do nothing
                // if no, make the update in the environment and add all dependent
                // assignments into the worklist and add cell to the execution list
            }

            // STEP 3 : Execute cells

            // split the mutability since we need mut access to just the prim
            // map
        } else if !non_comb_cells.is_empty() {
            let tmp = std::mem::take(&mut non_comb_cells);

            let new_assigns = eval_prims(
                &mut working_env,
                &dependency_map,
                tmp.iter().map(|x| x.into()),
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

fn eval_prims<'a, 'b, I: Iterator<Item = &'b RRC<ir::Cell>>>(
    env: &mut WorkingEnvironment,
    dependency_map: &DependencyMap<'a>,
    exec_list: I,
) -> HashSet<AssignmentRef<'a>> {
    let mut prim_map = std::mem::take(&mut env.backing_env.cell_prim_map);

    let mut update_list: Vec<(RRC<ir::Port>, OutputValue)> = vec![];
    let mut assign_list: HashSet<AssignmentRef> = HashSet::new();

    for cell in exec_list {
        let inputs = get_inputs(&env, &cell.borrow());

        let executable =
            prim_map.get_mut(&(&cell.borrow() as &ir::Cell as *const ir::Cell));

        if let Some(prim) = executable {
            let new_vals = prim.exec(&inputs);

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
            if val.as_u64() == 1 && val.vec.len() == 1 {
                true
            } else {
                panic!(
                    "Evaluating the truth value of a wire '{:?}' that is not one bit", p.borrow().canonical()
                )
            }
        }
        ir::Guard::True => true,
    }
}
