use crate::errors::FutilResult;
use crate::ir;
use std::collections::HashMap;
use std::rc::Rc;

/// The environment to interpret a FuTIL program
#[derive(Default, Clone)]
pub struct Environment {
    /// A mapping from cell names to the values on their ports.
    map: HashMap<ir::Id, HashMap<ir::Id, u64>>,
    /// A queue of operations that need to be applied in the future.
    /// A vector with maps that map cells to a mapping from ports to the new values.
    update_queue: Vec<HashMap<ir::Id, HashMap<ir::Id, u64>>>,
    /// Another implementation of the update queue
    /// Is a vector that pairs the source cell, the source port, the destination cell, and the destination port.
    updates: Vec<(ir::Id, ir::Id, ir::Id, ir::Id)>,
    /// A mapping from cell ids to cells, much like in component.rs. Will probably need to remove eventually
    cells: HashMap<ir::Id, ir::RRC<ir::Cell>>,
}

/// Helper functions for the environment.
impl Environment {
    /// Returns the value on a port, in a cell.
    pub fn get(&self, cell: &ir::Id, port: &ir::Id) -> u64 {
        self.map[cell][port]
    }

    /// Puts the mapping from cell to port to val in map.
    pub fn put(&mut self, cell: &ir::Id, port: &ir::Id, val: u64) -> () {
        let temp = self.map.get(cell).clone();

        if let Some(map) = temp {
            let mut mapcopy = map.clone();
            mapcopy.insert(port.clone(), val);
            self.map.insert(cell.clone(), mapcopy); // ???
        } else {
            let mut temp_map = HashMap::new();
            temp_map.insert(port.clone(), val);
            self.map.insert(cell.clone(), temp_map);
        }
    }

    /// Adds an update to the update queue; TODO; ok to drop prev and next?
    pub fn add_update(&mut self, prev: ir::Id, next: ir::Id, val: u64) -> () {
        let mut temp_map = HashMap::new();
        temp_map.insert(next, val);

        let mut outer_map = HashMap::new();
        outer_map.insert(prev, temp_map);

        self.update_queue.push(outer_map);
    }

    /// Puts an update into the updates vector
    pub fn put_update(
        &mut self,
        prev_cell: &ir::Id,
        prev_port: &ir::Id,
        next_cell: &ir::Id,
        next_port: &ir::Id,
    ) {
        self.updates.push((
            prev_cell.clone(),
            prev_port.clone(),
            next_cell.clone(),
            next_port.clone(),
        ));
    }

    /// Simulates a clock cycle by executing the stored updates.
    pub fn do_tick(&mut self) -> () {
        for updates in &self.updates.clone() {
            // read the values from the environment
            let new_val = self.get(&updates.0, &updates.1);
            self.put(&updates.2, &updates.3, new_val);
        }
        &self.updates.clear();
    }

    /// Performs an update to the current environment using the update_queue; TODO
    // pub fn do_tick(mut self) -> Self {
    //     for update in &self.update_queue.clone() {
    //         for cell in update.keys() {
    //             let port_val = &update[cell];
    //             for port in port_val.keys() {
    //                 self.put(cell, port, port_val[port]);
    //             }
    //         }
    //     }
    //     self
    // }

    /// Gets the cell based on the name; TODO; similar to find_cell in component.rs
    fn get_cell(&self, cell: &ir::Id) -> Option<ir::RRC<ir::Cell>> {
        self.cells
            .values()
            .find(|&g| g.borrow().name == *cell)
            .map(|r| Rc::clone(r))
    }
}

// Uses eval_assigns as a helper
fn eval_group(group: ir::Group, env: Environment) -> FutilResult<Environment> {
    let res = eval_assigns(&group.assignments, &env);
    res
}

// Evaluates assigns, given env; TODO
fn eval_assigns(
    assigns: &[ir::Assignment],
    env: &Environment,
) -> FutilResult<Environment> {
    // Find the done signal in the sequence of assignments
    let done_signal = get_done_signal(assigns);
    // e2 = Clone the current environment
    let mut write_env = env.clone();
    // get the cell that done_signal.dst belongs to
    let done_cell = get_cell(&done_signal.dst);

    // while done signal is zero; how to check value of done_signal?
    while env.get(&done_cell, &done_signal.dst.borrow().name) == 0 {
        // for assign in assigns
        for assign in assigns.iter() {
            // check if the assign.guard != 0
            if eval_guard(&assign.guard, env) {
                // check if the cells are constants?
                // cell of assign.src
                let src_cell = get_cell(&assign.src);
                // cell of assign.dst
                let dst_cell = get_cell(&assign.dst);

                // perform a read from `env` for assign.src
                let read_val =
                    env.get(&src_cell, &done_signal.src.borrow().name);

                // update internal state of the cell and
                // queue any required updates.

                // determine if dst_cell is a combinational cell or not
                if get_combinational_or_not(&dst_cell, env) {
                    // write to assign.dst to e2 immediately, if combinational
                    &write_env.put(
                        &dst_cell,
                        &assign.dst.borrow().name,
                        read_val,
                    );

                    // now, update the internal state of the cell; for now, this only includes adds; TODO
                    let mut inputs = vec![];
                    let mut outputs = vec![];

                    // get dst_cell's input vector
                    match &env.get_cell(&dst_cell) {
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
                    match &env.get_cell(&dst_cell) {
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


                    
                    match update_cell_state(
                        &dst_cell,
                        &inputs[..],
                        &outputs[..],
                        &write_env,
                    ) {
                        Ok(env) => write_env = env,
                        _ => println!("error in updating cell state"),
                    }
                } else {
                    // otherwise, add the write to the update queue; currently only handles registers
                    let temp_cell = &env.get_cell(&dst_cell);
                    match temp_cell {
                        Some(cell) => write_env.put_update(
                            &src_cell,
                            &assign.src.borrow().name,
                            &dst_cell,
                            &assign.dst.borrow().name,
                        ), //temp
                        // Some(cell) => write_env.add_update(
                        //     (cell.borrow()).name.clone(),
                        //     (cell.borrow()).get("in").borrow().name.clone(), //temp
                        //     1,                                               //temp
                        // ),
                        _ => panic!("can't find the ports"),
                    }
                }

                &write_env.do_tick();
            }
        }
    }

    Ok(write_env)
}

// used to convert guard's value to bool
fn eval_guard(guard: &ir::Guard, env: &Environment) -> bool {
    if eval_guard_helper(guard, env) != 0 {
        return true;
    } else {
        return false;
    }
}

/// Evaluates guard; TODO (messy u64 implementation)
fn eval_guard_helper(guard: &ir::Guard, env: &Environment) -> u64 {
    match guard {
        ir::Guard::Or(gs) => {
            for g in gs.clone() {
                if eval_guard_helper(&g, env) != 0 {
                    return 1;
                }
            }
            return 0;
        }
        ir::Guard::And(gs) => {
            for g in gs.clone() {
                if eval_guard_helper(&g, env) == 0 {
                    return 0;
                }
            }
            return 1;
        }
        ir::Guard::Eq(g1, g2) => {
            (eval_guard_helper(&**g1, env) == eval_guard_helper(&**g2, env))
                as u64
        }
        ir::Guard::Neq(g1, g2) => {
            (eval_guard_helper(&**g1, env) != eval_guard_helper(&**g2, env))
                as u64
        }
        ir::Guard::Gt(g1, g2) => {
            (eval_guard_helper(&**g1, env) > eval_guard_helper(&**g2, env))
                as u64
        }
        ir::Guard::Lt(g1, g2) => {
            (eval_guard_helper(&**g1, env) < eval_guard_helper(&**g2, env))
                as u64
        }
        ir::Guard::Geq(g1, g2) => {
            (eval_guard_helper(&**g1, env) >= eval_guard_helper(&**g2, env))
                as u64
        }
        ir::Guard::Leq(g1, g2) => {
            (eval_guard_helper(&**g1, env) <= eval_guard_helper(&**g2, env))
                as u64
        }
        ir::Guard::Not(g) => {
            if eval_guard_helper(g, &env) == 0 {
                return 1;
            } else {
                return 0;
            }
        }
        ir::Guard::Port(p) => env.get(&get_cell(p), &((*p.borrow()).name)),
        //TODO; this is probably the big one
        ir::Guard::True => 1,
    }
}

/// Get the cell a port belongs to.
/// Very similar to ir::Port::get_parent_name, except it can also panic
fn get_cell(port: &ir::RRC<ir::Port>) -> ir::Id {
    let id = ir::Port::get_parent_name(&(port.borrow()));
    // make sure that id is a cell id and not a group id; TODO
    id
}

/// Returns the done signal in this sequence of assignments
fn get_done_signal(assigns: &[ir::Assignment]) -> &ir::Assignment {
    for assign in assigns.iter() {
        // check if the statement's destination port is the "done" hole
        if (assign.dst.borrow()).name.id == "done".to_string() {
            return assign;
        }
    }
    panic!("no done signal");
}

/// Returns the done hole for a group
fn get_done_hole_group(group: &ir::Group) -> ir::RRC<ir::Port> {
    ir::Group::get(group, "done".to_string())
}

/// Determines if a cell is combinational or not. Will need to change implementation later.
fn get_combinational_or_not(cell: &ir::Id, env: &Environment) -> bool {
    // if cell is none,
    let cellg = env
        .get_cell(cell)
        .unwrap_or_else(|| panic!("Cannot find cell with name"));

    let cellgcopy = cellg.clone(); //??

    let cb = cellgcopy.borrow();

    let celltype = cb.type_name().unwrap_or_else(|| panic!("Constant?"));

    // TODO
    match (*celltype).id.as_str() {
        "std_reg" => false,
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
        | "fixed_p_std_add_dbit" => true,
        _ => false,
    }
}

/// Uses the cell's inputs ports to perform any required updates to the
/// cell's output ports. TODO
fn update_cell_state(
    cell: &ir::Id,
    inputs: &[ir::Id],
    output: &[ir::Id],
    env: &Environment, // should this be a reference
) -> FutilResult<Environment> {
    // get the actual cell, based on the id
    // let cell_r = cell.as_ref();

    let mut new_env = env.clone(); //??

    let cell_r = new_env
        .get_cell(cell)
        .unwrap_or_else(|| panic!("Cannot find cell with name"));

    let temp = cell_r.borrow(); //???

    // get the cell type
    let cell_type = temp.type_name();

    match cell_type {
        None => panic!("Futil Const?"),
        Some(ct) => match ct.id.as_str() {
            "std_reg" => {
                new_env.put(cell, &output[0], env.get(cell, &inputs[0]))
            }
            "std_add" | "std_" => new_env.put(
                cell,
                &output[0],
                new_env.get(cell, &inputs[0]) + env.get(cell, &inputs[1]),
            ),
            "std_mult" => new_env.put(
                cell,
                &output[0],
                new_env.get(cell, &inputs[0]) * env.get(cell, &inputs[1]),
            ),
            "std_not" => {
                new_env.put(cell, &output[0], !new_env.get(cell, &inputs[0]))
            }
            "std_and" => new_env.put(
                cell,
                &output[0],
                new_env.get(cell, &inputs[0]) & env.get(cell, &inputs[1]),
            ),
            "std_or" => new_env.put(
                cell,
                &output[0],
                new_env.get(cell, &inputs[0]) ^ env.get(cell, &inputs[1]),
            ),
            "std_gt" => new_env.put(
                cell,
                &output[0],
                (new_env.get(cell, &inputs[0]) > env.get(cell, &inputs[1]))
                    as u64,
            ),
            "std_lt" => new_env.put(
                cell,
                &output[0],
                (new_env.get(cell, &inputs[0]) > env.get(cell, &inputs[1]))
                    as u64,
            ),
            "std_eq" => new_env.put(
                cell,
                &output[0],
                (new_env.get(cell, &inputs[0]) == env.get(cell, &inputs[1]))
                    as u64,
            ),
            "std_neq" => new_env.put(
                cell,
                &output[0],
                (new_env.get(cell, &inputs[0]) != env.get(cell, &inputs[1]))
                    as u64,
            ),
            "std_ge" => new_env.put(
                cell,
                &output[0],
                (new_env.get(cell, &inputs[0]) >= env.get(cell, &inputs[1]))
                    as u64,
            ),
            "std_le" => new_env.put(
                cell,
                &output[0],
                (new_env.get(cell, &inputs[0]) <= env.get(cell, &inputs[1]))
                    as u64,
            ),
            "std_sqrt" => {
                //TODO; wrong implementation
                new_env.put(
                    cell,
                    &output[0],
                    ((new_env.get(cell, &inputs[0]) as f64).sqrt()) as u64, // cast to f64 to use sqrt
                );
            }
            _ => println!("ok"),
        },
    }

    // TODO
    Ok(new_env)
}
