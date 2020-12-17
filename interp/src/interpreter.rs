//! Interpreter "backend"

use calyx::{errors::FutilResult, ir};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
struct Update {
    /// The cell to be updated
    pub cell: ir::Id,
    /// The vector of input ports
    pub inputs: Vec<ir::Id>,
    /// The vector of output ports
    pub outputs: Vec<ir::Id>,
    /// Map of intermediate variables
    /// (could refer to a port or it could be "new", e.g. in the sqrt)
    pub vars: HashMap<String, u64>,
}

/// The environment to interpret a FuTIL program
#[derive(Clone, Debug)]
pub struct Environment {
    /// A mapping from cell names to the values on their ports.
    map: HashMap<ir::Id, HashMap<ir::Id, u64>>,
    /// A queue of operations that need to be applied in the future.
    /// A vector of Updates.
    update_queue: Vec<Update>,
    // XXX(karen): Will probably need to remove eventually
    /// A mapping from cell ids to cells, much like in component.rs.
    cells: HashMap<ir::Id, ir::RRC<ir::Cell>>,
}

/// Helper functions for the environment.
impl Environment {
    // Constructor "syntactic sugar"
    pub fn init(
        map: HashMap<ir::Id, HashMap<ir::Id, u64>>,
        cells: HashMap<ir::Id, ir::RRC<ir::Cell>>,
    ) -> Self {
        let update_queue: Vec<Update> = Vec::new();
        Self {
            map: map,
            update_queue: update_queue,
            cells: cells,
        }
    }

    /// Returns the value on a port, in a cell.
    pub fn get(&self, cell: &ir::Id, port: &ir::Id) -> u64 {
        self.map[cell][port]
    }

    /// Puts the mapping from cell to port to val in map.
    pub fn put(&mut self, cell: &ir::Id, port: &ir::Id, val: u64) -> () {
        let temp = self.map.get_mut(cell);

        if let Some(map) = temp {
            let mut mapcopy = map.clone();
            mapcopy.insert(port.clone(), val);
            //println!("mapcopy: {:?}", mapcopy);
            self.map.insert(cell.clone(), mapcopy);
        //println!("sefl.map: {:?}", self.map);
        } else {
            let mut temp_map = HashMap::new();
            temp_map.insert(port.clone(), val);
            self.map.insert(cell.clone(), temp_map);
        }
    }

    /// Adds an update to the update queue; TODO; ok to drop prev and next?
    pub fn add_update(
        &mut self,
        ucell: ir::Id,
        uinput: Vec<ir::Id>,
        uoutput: Vec<ir::Id>,
        uvars: HashMap<String, u64>,
    ) -> () {
        //println!("add update!");
        let update = Update {
            cell: ucell,
            inputs: uinput,
            outputs: uoutput,
            vars: uvars,
        };
        self.update_queue.push(update);
    }

    // // Convenience function to remove an update from the update queue
    // pub fn remove_update(&mut self, ucell: ir::Id) -> () {
    //     self.map.remove(&ucell); // this is wrong
    // }

    /// Simulates a clock cycle by executing the stored updates.
    pub fn do_tick(&mut self) -> () {
        //self.put(&ir::Id::from("reg0"), &ir::Id::from("done"), 1); //hard coding

        let uq = self.update_queue.clone();
        for update in uq {
            // println!("{:?}", update);
            let updated = update_cell_state(
                &update.cell,
                &update.inputs,
                &update.outputs,
                &self,
            );
            match updated {
                Ok(e) => {
                    //let m = self.map.get_mut(&update.cell);
                    let temp = e
                        .map
                        .get(&update.cell)
                        .unwrap_or_else(|| panic!("can't get map"))
                        .clone();
                    //m = temp;
                    self.map.insert(update.cell.clone(), temp);
                }
                _ => panic!("uh oh "),
            }
            //self.remove_update(update.cell); //?
        }
        // &self.updates.clear();
    }

    /// Gets the cell based on the name; TODO; similar to find_cell in component.rs
    fn get_cell(&self, cell: &ir::Id) -> Option<ir::RRC<ir::Cell>> {
        self.cells
            .values()
            .find(|&g| g.borrow().name == *cell)
            .map(|r| Rc::clone(r))
    }

    /// Outputs the cell state; TODO (write to a specified output in the future)
    pub fn cell_state(&self) {
        let state_str = self
            .map
            .iter()
            .map(|(cell, ports)| {
                format!(
                    "{}\n{}",
                    cell,
                    ports
                        .iter()
                        .map(|(p, v)| format!("\t{}: {}", p, v))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        println!("{}\n{}\n{}", "=".repeat(30), state_str, "=".repeat(30))
    }
}

/// Evaluates a group in an environment.
pub fn eval_group(
    group: ir::RRC<ir::Group>,
    env: Environment,
) -> FutilResult<Environment> {
    let g = group.borrow();

    let res = eval_assigns(&g.assignments, &env);
    res
}

/// Evaluates a group's assignment statements in an environment.
fn eval_assigns(
    assigns: &[ir::Assignment],
    env: &Environment,
) -> FutilResult<Environment> {
    // Find the done signal in the sequence of assignments
    let done_assign = get_done_signal(assigns);
    // e2 = Clone the current environment
    let mut write_env = env.clone();
    // get the cell that done_assign.src belongs to
    let done_cell = get_cell_from_port(&done_assign.src);

    // prevent infinite loops; should probably be deleted later (unless we want to display the clock cycle)?
    let mut counter = 0;

    // filter out the assignment statements that are not only from cells; for now, also excludes cells not in the env map
    let ok_assigns = assigns
        .iter()
        .filter(|&a| {
            is_cell(&a.dst.borrow())
                && is_cell(&a.dst.borrow())
                && env.map.contains_key(&get_cell_from_port(&a.src)) //dummy way of making sure the map has the a.src cell
                && env.map.contains_key(&get_cell_from_port(&a.dst))
            // ??
        })
        .collect::<Vec<_>>();

    // while done_assign src is 0 (done_assign.dst is not a cell's port; it should be a group's port)
    while write_env.get(&done_cell, &done_assign.src.borrow().name) == 0
        && counter < 5
    {
        //println!("Clock cycle {}", counter);
        /*println!(
            "state of done_cell {:1} : {:?} \n",
            &done_cell,
            write_env.map.get(&done_cell)
        );*/

        // for assign in assigns
        for assign in ok_assigns.iter() {
            // check if the assign.guard != 0
            if eval_guard(&assign.guard, env) {
                // check if the cells are constants?
                // cell of assign.src
                let src_cell = get_cell_from_port(&assign.src);
                // cell of assign.dst
                let dst_cell = get_cell_from_port(&assign.dst);

                /*println!(
                    "src cell {:1} port: {:2}, dest cell {:3} port: {:4}",
                    src_cell,
                    &assign.src.borrow().name,
                    dst_cell,
                    &assign.dst.borrow().name
                );*/

                // perform a read from `env` for assign.src
                let read_val = env.get(&src_cell, &assign.src.borrow().name);
                // println!("{}", read_val);

                // update internal state of the cell and
                // queue any required updates.

                //determine if dst_cell is a combinational cell or not
                if get_combinational_or_not(
                    &dst_cell,
                    &assign.dst.borrow().name,
                    env,
                ) {
                    // write to assign.dst to e2 immediately, if combinational
                    &write_env.put(
                        &dst_cell,
                        &assign.dst.borrow().name,
                        read_val,
                    );

                    /*println!(
                        "reg0.write_en = {}",
                        write_env.get(
                            &ir::Id::from("reg0"),
                            &ir::Id::from("write_en")
                        )
                    );*/

                    // now, update the internal state of the cell; for now, this only includes adds; TODO (use primitive Cell parameters)
                    let inputs;
                    let outputs;

                    // TODO: hacky way to avoid updating the cell state. Also, how to get input and output vectors in general??
                    if &assign.dst.borrow().name != "write_en" {
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

                        // update the cell state in write_env
                        write_env = update_cell_state(
                            &dst_cell,
                            &inputs[..],
                            &outputs[..],
                            &write_env,
                        )?;
                    }
                } else {
                    // otherwise, add the write to the update queue; currently only handles registers

                    // get input and output vectors; TODO (currently only works for registers)
                    // println!("src: {}", src_cell);
                    // println!("src port: {}", &assign.src.borrow().name);
                    // println!("dst port: {}", &assign.dst.borrow().name);

                    // get input cell
                    let inputs = vec![src_cell.clone()];
                    // get dst_cell's output port
                    let outputs = vec![assign.dst.borrow().name.clone()];

                    write_env =
                        init_cells(&dst_cell, inputs, outputs, write_env)?;
                }
            }
        }
        //println!("do tick");
        &write_env.do_tick();
        //println!("done with tick");
        counter += 1;
    }

    /*println!(
        "\nFinal state of the done cell, i.e. {:1}: {:?} \n",
        &done_cell,
        write_env.map.get(&done_cell)
    );*/
    Ok(write_env)
}

/// Convenience function to determine if a port's parent is a cell or not
fn is_cell(port: &ir::Port) -> bool {
    match &port.parent {
        ir::PortParent::Cell(_) => true,
        _ => false,
    }
}

/// Evalutes a guard in an environment.
fn eval_guard(guard: &ir::Guard, env: &Environment) -> bool {
    if eval_guard_helper(guard, env) != 0 {
        return true;
    } else {
        return false;
    }
}

/// Evaluate guard implementation; TODO (messy u64 implementation?)
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
        ir::Guard::Port(p) => {
            env.get(&get_cell_from_port(p), &((*p.borrow()).name))
        }
        //TODO; this is probably the big one
        ir::Guard::True => 1,
    }
}

/// Get the cell id a port belongs to.
/// Very similar to ir::Port::get_parent_name, except it can also panic
fn get_cell_from_port(port: &ir::RRC<ir::Port>) -> ir::Id {
    if is_cell(&port.borrow()) {
        return ir::Port::get_parent_name(&(port.borrow()));
    } else {
        panic!("port belongs to a group, not a cell!");
    }
}

/// Returns the assignment statement with the done signal; assumes there aren't other groups to check?
fn get_done_signal(assigns: &[ir::Assignment]) -> &ir::Assignment {
    for assign in assigns.iter() {
        let dest = assign.dst.borrow();
        // need to check g's name?
        let group_or_not = match &dest.parent {
            ir::PortParent::Group(_) => true,
            _ => false,
        };
        // check if the statement's destination port is the "done" hole and if its parent is a group
        if dest.name.id == "done".to_string() && group_or_not {
            return assign;
        }
    }
    unreachable!("Group does not have a done signal");
}

/// Determines if writing a particular cell and cell port is combinational or not. Will need to change implementation later.
fn get_combinational_or_not(
    cell: &ir::Id,
    port: &ir::Id,
    env: &Environment,
) -> bool {
    // if cell is none,
    let cellg = env
        .get_cell(cell)
        .unwrap_or_else(|| panic!("Cannot find cell with name"));

    let cb = cellg.borrow();
    let celltype = cb.type_name().unwrap_or_else(|| panic!("Constant?"));

    // TODO; get cell attributes
    match (*celltype).id.as_str() {
        "std_reg" => match (*port).id.as_str() {
            "write_en" => true,
            "out" => false,
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
        _ => false,
    }
}

// Initializes values for the update queue, i.e. for non-combinational cells
fn init_cells(
    cell: &ir::Id,
    inputs: Vec<ir::Id>,
    outputs: Vec<ir::Id>,
    mut env: Environment,
) -> FutilResult<Environment> {
    //let mut new_env = env.clone();

    let cell_r = env
        .get_cell(cell)
        .unwrap_or_else(|| panic!("Cannot find cell with name"));

    // get the cell type
    match cell_r.borrow().type_name() {
        None => panic!("bad"),
        Some(ct) => match ct.id.as_str() {
            "std_sqrt" => { //:(
                // has intermediate steps/computation??
            },
            "std_reg" => {
                let map : HashMap<String, u64> = HashMap::new(); //placeholder
                // reg.in = dst port should go here
                env.add_update(cell.clone(), inputs, outputs, map);
            }
            _ => panic!("attempted to initalize an update queue map for a combinational cell")
        }
    }

    Ok(env)
}

/// Uses the cell's inputs ports to perform any required updates to the
/// cell's output ports.
/// TODO: how to get input and output ports in general? How to "standardize" for combinational or not operations
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
    let cell_type = temp.type_name().unwrap_or_else(|| panic!("Futil Const?"));

    match cell_type.id.as_str() {
        "std_reg" => {
            let write_en = ir::Id::from("write_en");

            // if (){

            // }
            // register's write_en must be high to write reg.out and reg.done
            if new_env.get(&cell, &write_en) != 0 {
                //println!("reg update");
                let out = ir::Id::from("out"); //assuming reg.in = cell.out, always
                let inp = ir::Id::from("in"); //assuming reg.in = cell.out, always
                let done = ir::Id::from("done"); //done id

                // println!("cell to read from: {}", inputs[0]);
                // println!("reg port to write to: {}", &output[0]);
                // println!("value to write to cell port: {}", env.get(&inputs[0], &out));

                new_env.put(cell, &output[0], env.get(&inputs[0], &out)); //reg.in = cell.out; should this be in init?

                if output[0].id == "in" {
                    new_env.put(cell, &out, new_env.get(cell, &inp)); // reg.out = reg.in
                    new_env.put(cell, &done, 1); // reg.done = 1'd1
                                                 // remove from update queue
                                                 //new_env.remove_update((*cell).clone()); // check the type of cell
                }
            }
        }
        "std_sqrt" => {
            //TODO; wrong implementation
            new_env.put(
                cell,
                &output[0],
                ((new_env.get(cell, &inputs[0]) as f64).sqrt()) as u64, // cast to f64 to use sqrt
            );
        }
        "std_add" => new_env.put(
            cell,
            &output[0],
            new_env.get(cell, &inputs[0]) + env.get(cell, &inputs[1]),
        ),
        "std_sub" => new_env.put(
            cell,
            &output[0],
            new_env.get(cell, &inputs[0]) - env.get(cell, &inputs[1]),
        ),
        "std_mod" => new_env.put(
            cell,
            &output[0],
            new_env.get(cell, &inputs[0]) % env.get(cell, &inputs[1]),
        ),
        "std_mult" => new_env.put(
            cell,
            &output[0],
            new_env.get(cell, &inputs[0]) * env.get(cell, &inputs[1]),
        ),
        "std_div" => {
            // need this condition to avoid divide by 0
            // (e.g. if only one of left/right ports has been updated from the initial nonzero value?)
            // TODO: what if the program specifies a divide by 0? how to catch??
            if env.get(cell, &inputs[1]) != 0 {
                new_env.put(
                    cell,
                    &output[0],
                    new_env.get(cell, &inputs[0]) / env.get(cell, &inputs[1]),
                )
            }
        }
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
            (new_env.get(cell, &inputs[0]) > env.get(cell, &inputs[1])) as u64,
        ),
        "std_lt" => new_env.put(
            cell,
            &output[0],
            (new_env.get(cell, &inputs[0]) > env.get(cell, &inputs[1])) as u64,
        ),
        "std_eq" => new_env.put(
            cell,
            &output[0],
            (new_env.get(cell, &inputs[0]) == env.get(cell, &inputs[1])) as u64,
        ),
        "std_neq" => new_env.put(
            cell,
            &output[0],
            (new_env.get(cell, &inputs[0]) != env.get(cell, &inputs[1])) as u64,
        ),
        "std_ge" => new_env.put(
            cell,
            &output[0],
            (new_env.get(cell, &inputs[0]) >= env.get(cell, &inputs[1])) as u64,
        ),
        "std_le" => new_env.put(
            cell,
            &output[0],
            (new_env.get(cell, &inputs[0]) <= env.get(cell, &inputs[1])) as u64,
        ),
        _ => unimplemented!("{}", cell_type),
    }

    // TODO
    Ok(new_env)
}
