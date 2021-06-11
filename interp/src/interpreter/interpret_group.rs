//! Used for the command line interface.
//! Only interprets a given group in a given component

use crate::primitives::{
    Execute, ExecuteBinary, ExecuteStateful, ExecuteUnary,
};
use crate::utils::{AssignmentRef, OutputValueRef};
use crate::values::{OutputValue, TimeLockedValue, Value};
use crate::{
    environment::Environment, environment::UpdateQueue, primitives,
    primitives::Primitive,
};
use calyx::{
    errors::{Error, FutilResult},
    ir::{self, CloneName, RRC},
};
use std::collections::{HashMap, HashSet};
use std::iter;
use std::rc::Rc;
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
                self.map
                    .entry(&port.borrow() as &ir::Port as *const ir::Port)
                    .or_default()
                    .insert(assignment.into());
            }
        }
    }

    fn get(&self, port: &ir::Port) -> Option<&HashSet<AssignmentRef<'a>>> {
        self.map.get(&(port as *const ir::Port))
    }
}

type WorkList<'a> = HashSet<AssignmentRef<'a>>;

type PortOutputValMap = HashMap<*const ir::Port, OutputValue>;

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
}

// possibly #[inline] here later? Compiler probably knows to do that already
fn get_done_port(group: &ir::Group) -> RRC<ir::Port> {
    group.get(&"done")
}

/// Get the name of the component to interpret from the context.
fn _get_component(
    ctx: ir::Context,
    component: &str,
) -> FutilResult<ir::Component> {
    match ctx.components.into_iter().find(|c| c.name.id == *component) {
        Some(comp) => Ok(comp),
        None => Err(Error::Undefined(
            ir::Id::from(component.to_string()),
            "component".to_string(),
        )),
    }
}

/// Construct a map from cell ids to a map from the cell's ports' ids to the ports' values
fn _construct_map(
    cells: &[ir::RRC<ir::Cell>],
) -> HashMap<ir::Id, HashMap<ir::Id, u64>> {
    let mut map = HashMap::new();
    for cell in cells {
        let cb = cell.borrow();
        let mut ports: HashMap<ir::Id, u64> = HashMap::new();

        match &cb.prototype {
            // A Calyx constant cell's out port is that constant's value
            ir::CellType::Constant { val, .. } => {
                ports.insert(ir::Id::from("out"), *val);
                map.insert(cb.clone_name(), ports);
            }
            ir::CellType::Primitive { .. } => {
                for port in &cb.ports {
                    // All ports for primitives are initalized to 0 , unless the cell is an std_const
                    let pb = port.borrow();
                    let initval = cb
                        .get_paramter(&ir::Id::from("value".to_string()))
                        .unwrap_or(0); //std_const should be the only cell type with the "value" parameter

                    ports.insert(pb.name.clone(), initval);
                }
                map.insert(cb.clone_name(), ports);
            }
            _ => panic!("component"),
        }
    }
    map
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
    mut env: Environment,
) -> FutilResult<Environment> {
    let mut dependency_map =
        DependencyMap::from_assignments(group.assignments.iter());
    let grp_done = get_done_port(&group);
    let mut working_env: WorkingEnvironment = env.into();
    let mut worklist: WorkList =
        group.assignments.iter().map(|x| x.into()).collect();

    while !grp_is_done(working_env.get(&grp_done.borrow())) {
        if !worklist.is_empty() {
            let mut updates_list = vec![];
            let mut exec_list: Vec<RRC<ir::Cell>> = vec![];

            // STEP 1 : Evaluate all assignments
            for assignment in worklist.drain() {
                if eval_guard(&assignment.guard, &working_env) {
                    updates_list.push((
                        Rc::clone(&assignment.src),
                        working_env.get(&assignment.dst.borrow()),
                    ));
                }
            }

            // STEP 2 : Update values and determine new worklist and exec_list
            for (port, new_val) in updates_list {
                let current_val = working_env.get(&port.borrow());
                // check if the current val of id matches the new update
                // if yes, do nothing
                // if no, make the update in the environment and add all dependent
                // assignments into the worklist and add cell to the execution list
                if current_val != new_val {
                    let cell = match &port.borrow().parent {
                        ir::PortParent::Cell(c) => Some(c.upgrade()),
                        ir::PortParent::Group(_) => None,
                    };
                    let new_assigments = dependency_map.get(&port.borrow());

                    if cell.is_some() {
                        exec_list.push(cell.unwrap());
                    }

                    if new_assigments.is_some() {
                        worklist
                            .extend(new_assigments.unwrap().iter().cloned());
                    }
                }
            }

            // STEP 3 : Execute cells
            let mut prim_map =
                std::mem::take(&mut working_env.backing_env.cell_prim_map);

            for cell in exec_list {
                let inputs: Vec<(ir::Id, &Value)> = cell
                    .borrow()
                    .ports
                    .iter()
                    .filter_map(|p| {
                        let p_ref: &ir::Port = &p.borrow();
                        match &p_ref.direction {
                            ir::Direction::Input => Some((
                                p_ref.name.clone(),
                                working_env.get_as_val(p_ref),
                            )),
                            _ => None,
                        }
                    })
                    .collect();

                let new_vals = prim_map
                    .get_mut(&(&cell.borrow() as &ir::Cell as *const ir::Cell))
                    .unwrap()
                    .exec(&inputs);

                std::mem::drop(inputs);

                for (port, val) in new_vals {
                    let port_ref = cell.borrow().find(port).unwrap();

                    let current_val = working_env.get(&port_ref.borrow());

                    if current_val != (&val).into() {
                        working_env.update_val(&port_ref.borrow(), val);
                        let new_assigments =
                            dependency_map.get(&port_ref.borrow());

                        if new_assigments.is_some() {
                            worklist.extend(
                                new_assigments.unwrap().iter().cloned(),
                            );
                        }
                    }
                }
            }

            working_env.backing_env.cell_prim_map = prim_map;
        } else {
            // tick clock
        }
    }

    todo!()
}

// XXX(karen): I think it will need another copy of environment for each
// iteration of assignment statements
/// Evaluates a group's assignment statements in an environment.
/// How this is done:
/// First, a new write-to environment is cloned from the original read-only environment.
/// For each clock cycle (until the group's done signal is high):
/// Then, each assignment statement is checked for its done signal is high.
/// If that statement's done signal is high:
/// If the assignment is combinational, it is immediately evaluated result and stored in the write-to environment.
/// If it is not combinational, then it is added to an update queue, to be evaluated at the end of the current clock cycle.
/// This continues until the group's done signal is high.
fn eval_assigns(
    assigns: &[ir::Assignment],
    mut env: Environment,
    component: &ir::Id,
) -> FutilResult<Environment> {
    todo!()
    // // Find the done signal in the sequence of assignments
    // let done_assign = get_done_signal(assigns);

    // // Clone the current environment
    // let mut write_env = env.clone();

    // // XXX: Prevent infinite loops. should probably be deleted later
    // // (unless we want to display the clock cycle)?
    // let mut counter = 0;

    // // Filter out the assignment statements that are not only from cells.
    // // Reorder assignment statements??
    // // XXX: for now, also excludes cells not in the env map
    // let ok_assigns = assigns
    //     .iter()
    //     .filter(|&a| {
    //         !a.dst.borrow().is_hole()
    //             // dummy way of making sure the map has the a.src cell
    //             && env.get_cell(&component, &get_cell_from_port(&a.src)).is_some()
    //             && env.get_cell(&component, &get_cell_from_port(&a.dst)).is_some()
    //     })
    //     .collect::<Vec<_>>();

    // // XXX(yoona): At the moment interpreter rejects direct assignment of 1 to the groups
    // // needs to be fixed
    // if write_env.get_from_port(&component, &done_assign.src.borrow()) == 1 {
    //     panic!("TODO: done[group]=1 this group woud but be evaluated ");
    // }

    // // While done_assign.src is 0
    // // (we use done_assign.src because done_assign.dst is not a cell's port; it should be a group's port

    // while write_env.get_from_port(&component, &done_assign.src.borrow()) == 0
    //     && counter < 5
    // {
    //     env = write_env.clone();
    //     // println!("Clock cycle {}", counter);

    //     // Update queue for staging updates
    //     let mut uq = UpdateQueue::init(component.clone());

    //     // Iterate through assignment statements
    //     for assign in &ok_assigns {
    //         // check if the assign.guard != 0
    //         // should it be evaluating the guard in write_env environment?
    //         if eval_guard(&component, &assign.guard, &write_env) != 0 {
    //             // check if the cells are constants?
    //             // cell of assign.src
    //             let src_cell = get_cell_from_port(&assign.src);
    //             // cell of assign.dst
    //             let dst_cell = get_cell_from_port(&assign.dst);

    //             // perform a read from `env` for assign.src
    //             // XXX(karen): should read from the previous iteration's env?
    //             let read_val =
    //                 env.get_from_port(&component, &assign.src.borrow());

    //             // update internal state of the cell and
    //             // queue any required updates.

    //             //determine if dst_cell is a combinational cell or not.
    //             // If so, it should be immediately evaluated and stored.
    //             if is_combinational(
    //                 &component,
    //                 &dst_cell,
    //                 &assign.dst.borrow().name,
    //                 &env,
    //             ) {
    //                 write_env.put(
    //                     &component,
    //                     &dst_cell,
    //                     &assign.dst.borrow().name,
    //                     read_val,
    //                 );

    //                 // now, update the internal state of the cell;
    //                 // for now, this only includes cells with left and right ports;
    //                 // TODO (use primitive Cell parameters)
    //                 let inputs;
    //                 let outputs;

    //                 // TODO: hacky way to avoid updating the cell state.
    //                 // Also, how to get input and output vectors in general??
    //                 if &assign.dst.borrow().name != "write_en" {
    //                     // get dst_cell's input vector
    //                     match &write_env.get_cell(&component, &dst_cell) {
    //                         Some(cell) => {
    //                             inputs = vec![
    //                                 (cell.borrow())
    //                                     .get("left")
    //                                     .borrow()
    //                                     .name
    //                                     .clone(),
    //                                 (cell.borrow())
    //                                     .get("right")
    //                                     .borrow()
    //                                     .name
    //                                     .clone(),
    //                             ]
    //                         }
    //                         _ => panic!("could not find cell"),
    //                     }

    //                     // get dst_cell's output vector
    //                     match &write_env.get_cell(&component, &dst_cell) {
    //                         Some(cell) => {
    //                             outputs = vec![(cell.borrow())
    //                                 .get("out")
    //                                 .borrow()
    //                                 .name
    //                                 .clone()]
    //                             //clean this up later?
    //                         }
    //                         _ => panic!("could not find cell"),
    //                     }

    //                     // update the cell state in write_env
    //                     write_env = primitives::update_cell_state(
    //                         &dst_cell,
    //                         &inputs,
    //                         &outputs,
    //                         &write_env,
    //                         component.clone(),
    //                     )?;
    //                 }
    //             } else {
    //                 // otherwise, add the write to the update queue; currently only handles registers

    //                 // get input cell
    //                 let inputs = vec![src_cell.clone()];
    //                 // get dst_cell's output port
    //                 let outputs = vec![assign.dst.borrow().name.clone()];

    //                 uq = uq.init_cells(
    //                     &dst_cell,
    //                     inputs,
    //                     outputs,
    //                     write_env.clone(),
    //                 );
    //             }
    //         }
    //     }
    //     write_env = uq.do_tick(write_env.clone())?;
    //     counter += 1;
    // }
    // Ok(write_env)
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
                    "Evaluating the truth value of a wire that is not one bit"
                )
            }
        }
        ir::Guard::True => true,
    }
}

/// Get the cell id a port belongs to.
/// Very similar to ir::Port::get_parent_name, except it can also panic
fn get_cell_from_port(port: &ir::RRC<ir::Port>) -> ir::Id {
    if port.borrow().is_hole() {
        panic!("Unexpected hole. Cannot get cell: {}", port.borrow().name)
    }
    port.borrow().get_parent_name()
}

/// Returns the assignment statement with the done signal; assumes there aren't other groups to check?
fn get_done_signal(assigns: &[ir::Assignment]) -> &ir::Assignment {
    assigns
        .iter()
        .find(|assign| {
            let dst = assign.dst.borrow();
            dst.is_hole() && dst.name == "done"
        })
        .expect("Group does not have a done signal")
}

/// Determines if writing a particular cell and cell port is combinational or not.
/// Will need to change implementation later.
fn is_combinational(
    component: &ir::Id,
    cell: &ir::Id,
    port: &ir::Id,
    env: &Environment,
) -> bool {
    // if cell is none,
    let cellg = env
        .get_cell(component, cell)
        .unwrap_or_else(|| panic!("Cannot find cell with name"));

    let cb = cellg.borrow();
    let celltype = cb.type_name().unwrap_or_else(|| panic!("Constant?"));

    // TODO; get cell attributes
    match (*celltype).id.as_str() {
        "std_reg" => match port.id.as_str() {
            // XXX(rachit): Why is this a "combinational" port?
            "write_en" => true,
            "out" => false,
            "done" => false,
            _ => false,
        },
        "std_mem_d1" => match port.id.as_str() {
            "write_en" => true,
            "read_data" => false,
            "done" => false,
            _ => false,
        },
        "std_const"
        | "std_slice"
        | "std_lsh"
        | "std_rsh"
        | "std_add"
        | "std_sub"
        | "std_mod"
        | "std_mult"
        | "std_div"
        | "std_not"
        | "std_and"
        | "std_or"
        | "std_xor"
        | "std_gt"
        | "std_lt"
        | "std_eq"
        | "std_neq"
        | "std_ge"
        | "std_le"
        | "fixed_p_std_const"
        | "fixed_p_std_add"
        | "fixed_p_std_sub"
        | "fixed_p_std_mult"
        | "fixed_p_std_div"
        | "fixed_p_std_gt"
        | "fixed_p_std_add_dbit" => true,
        prim => panic!("unknown primitive {}", prim),
    }
}
