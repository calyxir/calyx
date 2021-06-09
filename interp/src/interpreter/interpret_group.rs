//! Used for the command line interface.
//! Only interprets a given group in a given component

use crate::{environment::Environment, primitives, update::UpdateQueue};
use calyx::{
    errors::{Error, FutilResult},
    ir::{self, CloneName, RRC},
};
use std::collections::{HashMap, HashSet};
use std::iter;
#[derive(Debug, Clone, Default)]
struct DependencyMap<'a> {
    map: HashMap<*const ir::Port, HashSet<&'a ir::Assignment>>,
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
                    .insert(assignment);
            }
        }
    }
}

type WorkList<'a> = HashSet<&'a ir::Assignment>;

fn get_done_port(group: &ir::Group) -> RRC<ir::Port> {
    group.find(&"done").unwrap()
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

/// Evaluates a group, given an environment.
pub fn interpret_group(
    group: &ir::Group,
    env: Environment,
    component: &ir::Id,
) -> FutilResult<Environment> {
    eval_assigns(&group.assignments, env, component)
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
    // Find the done signal in the sequence of assignments
    let done_assign = get_done_signal(assigns);

    // Clone the current environment
    let mut write_env = env.clone();

    // XXX: Prevent infinite loops. should probably be deleted later
    // (unless we want to display the clock cycle)?
    let mut counter = 0;

    // Filter out the assignment statements that are not only from cells.
    // Reorder assignment statements??
    // XXX: for now, also excludes cells not in the env map
    let ok_assigns = assigns
        .iter()
        .filter(|&a| {
            !a.dst.borrow().is_hole()
                // dummy way of making sure the map has the a.src cell
                && env.get_cell(&component, &get_cell_from_port(&a.src)).is_some()
                && env.get_cell(&component, &get_cell_from_port(&a.dst)).is_some()
        })
        .collect::<Vec<_>>();

    // XXX(yoona): At the moment interpreter rejects direct assignment of 1 to the groups
    // needs to be fixed
    if write_env.get_from_port(&component, &done_assign.src.borrow()) == 1 {
        panic!("TODO: done[group]=1 this group woud but be evaluated ");
    }

    // While done_assign.src is 0
    // (we use done_assign.src because done_assign.dst is not a cell's port; it should be a group's port

    while write_env.get_from_port(&component, &done_assign.src.borrow()) == 0
        && counter < 5
    {
        env = write_env.clone();
        // println!("Clock cycle {}", counter);

        // Update queue for staging updates
        let mut uq = UpdateQueue::init(component.clone());

        // Iterate through assignment statements
        for assign in &ok_assigns {
            // check if the assign.guard != 0
            // should it be evaluating the guard in write_env environment?
            if eval_guard(&component, &assign.guard, &write_env) != 0 {
                // check if the cells are constants?
                // cell of assign.src
                let src_cell = get_cell_from_port(&assign.src);
                // cell of assign.dst
                let dst_cell = get_cell_from_port(&assign.dst);

                // perform a read from `env` for assign.src
                // XXX(karen): should read from the previous iteration's env?
                let read_val =
                    env.get_from_port(&component, &assign.src.borrow());

                // update internal state of the cell and
                // queue any required updates.

                //determine if dst_cell is a combinational cell or not.
                // If so, it should be immediately evaluated and stored.
                if is_combinational(
                    &component,
                    &dst_cell,
                    &assign.dst.borrow().name,
                    &env,
                ) {
                    write_env.put(
                        &component,
                        &dst_cell,
                        &assign.dst.borrow().name,
                        read_val,
                    );

                    // now, update the internal state of the cell;
                    // for now, this only includes cells with left and right ports;
                    // TODO (use primitive Cell parameters)
                    let inputs;
                    let outputs;

                    // TODO: hacky way to avoid updating the cell state.
                    // Also, how to get input and output vectors in general??
                    if &assign.dst.borrow().name != "write_en" {
                        // get dst_cell's input vector
                        match &write_env.get_cell(&component, &dst_cell) {
                            Some(cell) => {
                                inputs = vec![
                                    (cell.borrow())
                                        .get("left")
                                        .borrow()
                                        .name
                                        .clone(),
                                    (cell.borrow())
                                        .get("right")
                                        .borrow()
                                        .name
                                        .clone(),
                                ]
                            }
                            _ => panic!("could not find cell"),
                        }

                        // get dst_cell's output vector
                        match &write_env.get_cell(&component, &dst_cell) {
                            Some(cell) => {
                                outputs = vec![(cell.borrow())
                                    .get("out")
                                    .borrow()
                                    .name
                                    .clone()]
                                //clean this up later?
                            }
                            _ => panic!("could not find cell"),
                        }

                        // update the cell state in write_env
                        write_env = primitives::update_cell_state(
                            &dst_cell,
                            &inputs,
                            &outputs,
                            &write_env,
                            component.clone(),
                        )?;
                    }
                } else {
                    // otherwise, add the write to the update queue; currently only handles registers

                    // get input cell
                    let inputs = vec![src_cell.clone()];
                    // get dst_cell's output port
                    let outputs = vec![assign.dst.borrow().name.clone()];

                    uq = uq.init_cells(
                        &dst_cell,
                        inputs,
                        outputs,
                        write_env.clone(),
                    );
                }
            }
        }
        write_env = uq.do_tick(write_env.clone())?;
        counter += 1;
    }
    Ok(write_env)
}

/// Evaluate guard implementation
#[allow(clippy::borrowed_box)]
// XXX: Allow for this warning. It would make sense to use a reference when we
// have the `box` match pattern available in Rust.
fn eval_guard(comp: &ir::Id, guard: &Box<ir::Guard>, env: &Environment) -> u64 {
    (match &**guard {
        ir::Guard::Or(g1, g2) => {
            (eval_guard(comp, g1, env) == 1) || (eval_guard(comp, g2, env) == 1)
        }
        ir::Guard::And(g1, g2) => {
            (eval_guard(comp, g1, env) == 1) && (eval_guard(comp, g2, env) == 1)
        }
        ir::Guard::Not(g) => eval_guard(comp, g, &env) != 0,
        ir::Guard::Eq(g1, g2) => {
            env.get_from_port(comp, &g1.borrow())
                == env.get_from_port(comp, &g2.borrow())
        }
        ir::Guard::Neq(g1, g2) => {
            env.get_from_port(comp, &g1.borrow())
                != env.get_from_port(comp, &g2.borrow())
        }
        ir::Guard::Gt(g1, g2) => {
            env.get_from_port(comp, &g1.borrow())
                > env.get_from_port(comp, &g2.borrow())
        }
        ir::Guard::Lt(g1, g2) => {
            env.get_from_port(comp, &g1.borrow())
                < env.get_from_port(comp, &g2.borrow())
        }
        ir::Guard::Geq(g1, g2) => {
            env.get_from_port(comp, &g1.borrow())
                >= env.get_from_port(comp, &g2.borrow())
        }
        ir::Guard::Leq(g1, g2) => {
            env.get_from_port(comp, &g1.borrow())
                <= env.get_from_port(comp, &g2.borrow())
        }
        ir::Guard::Port(p) => env.get_from_port(comp, &p.borrow()) != 0,
        ir::Guard::True => true,
    }) as u64
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
