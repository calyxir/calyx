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
    update_queue: Vec<HashMap<ir::Id, HashMap<ir::Id, u64>>>,
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

    /// Adds an update to the update queue; TODO
    pub fn add_update(&self) -> () {}

    /// Performs an update to the current environment using the update_queue; TODO
    pub fn do_tick(self) -> Self {
        for update in &self.update_queue {
            println!("test")
        }
        self
    }
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
    //let init = done_signal.src.clone();

    // e2 = Clone the current environment
    let mut write_env = env.clone();

    // get the cell that done_signal.dst belongs to
    let cell = get_cell(&done_signal.dst);

    // while done signal is zero; how to check value of done_signal?
    while env.get(&cell, &done_signal.dst.borrow().name) == 0 {}
    // for assign in assigns
    for assign in assigns.iter() {
        // check if the assign.guard == 1
        if eval_guard(&assign.guard) {
            // cell of assign.src
            let src_cell = get_cell(&assign.src);
            // cell of assign.dst
            let dst_cell = get_cell(&assign.dst);

            // perform a read from `env` for assign.src
            let read_val = env.get(&src_cell, &done_signal.src.borrow().name);

            // update internal state of the cell and
            // queue any required updates.

            // determine if src is a combinational cell or not
            if get_combinational_or_not(&src_cell, env) {
                // write to assign.dst to e2 immediately, if combinational
                write_env.put(
                    &dst_cell,
                    &done_signal.dst.borrow().name,
                    read_val,
                );
            } else {
                // otherwise, add the write to the update queue
            }

            // env = env.do_tick()
        }
    }

    Ok(write_env)
}

/// Evaluates guard; TODO
fn eval_guard(guard: &ir::Guard) -> bool {
    match guard {
        ir::Guard::True => true,
        ir::Guard::Port(p) => true, //TODO; this is probably the big one
        ir::Guard::Not(g) => !(eval_guard(g)),
        _ => true,
    }
}

/// Get the cell a port belongs to.
/// Very similar to ir::Port::get_parent_name, except it can also panic
fn get_cell(dest: &ir::RRC<ir::Port>) -> ir::Id {
    let id = ir::Port::get_parent_name(&(dest.borrow()));
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
        "std_add" => true,
        "std_reg" => false,
        "std_const" => true,
        _ => false,
    }
}

/// Uses the cell's inputs ports to perform any required updates to the
/// cell's output ports. TODO
fn update_cell_state(
    cell: &ir::Id,
    inputs: &[ir::Id],
    output: &[ir::Id],
    env: Environment,
) -> FutilResult<Environment> {
    // get the actual cell, based on the id
    // let cell_r = cell.as_ref();

    let mut e = env.clone(); //??

    let cell_r = e
        .get_cell(cell)
        .unwrap_or_else(|| panic!("Cannot find cell with name"));

    let temp = cell_r.borrow(); //???

    // get the cell type
    let cell_type = temp.type_name();

    match cell_type {
        None => println!("Futil Const?"),
        Some(ct) => match ct.id.as_str() {
            "std_add" =>
            // let a = e.get(cell, inputs[0]);
            // let b = e.get(cell, inputs[1]);
            {
                e.put(
                    cell,
                    &output[0],
                    e.get(cell, &inputs[0]) + e.get(cell, &inputs[1]),
                )
            }
            _ => println!("ok"),
        },
    }

    // TODO
    Ok(e)
}
