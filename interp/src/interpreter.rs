use super::{environment::Environment, primitives};
use calyx::{errors::FutilResult, ir};
use std::collections::HashMap;

/// Evaluates a group, given an environment.
pub fn eval_group(
    group: ir::RRC<ir::Group>,
    env: &Environment,
) -> FutilResult<Environment> {
    eval_assigns(&(group.borrow()).assignments, &env)
}

// XXX(karen): I think it will need another copy of environment for each
// iteration of assignment statements
/// Evaluates a group's assignment statements in an environment.
fn eval_assigns(
    assigns: &[ir::Assignment],
    env: &Environment,
) -> FutilResult<Environment> {
    // Find the done signal in the sequence of assignments
    let done_assign = get_done_signal(assigns);

    // e2 = Clone the current environment
    let mut write_env = env.clone();

    // XXX: Prevent infinite loops. should probably be deleted later
    // (unless we want to display the clock cycle)?
    let mut counter = 0;

    // Filter out the assignment statements that are not only from cells.
    // XXX: for now, also excludes cells not in the env map
    let ok_assigns = assigns
        .iter()
        .filter(|&a| {
            !a.dst.borrow().is_hole()
                // dummy way of making sure the map has the a.src cell
                && env.get_cell(&get_cell_from_port(&a.src)).is_some()
                && env.get_cell(&get_cell_from_port(&a.dst)).is_some()
        })
        .collect::<Vec<_>>();

    // While done_assign.src is 0 (we use done_assign.src because done_assign.dst is not a cell's port; it should be a group's port)
    while write_env.get_from_port(&done_assign.src.borrow()) == 0 && counter < 5
    {
        // println!("Clock cycle {}", counter);
        /*println!(
            "state of done_cell {:1} : {:?} \n",
            &done_cell,
            write_env.map.get(&done_cell)
        );*/
        // "staging" updates
        //let mut iter_updates = write_env.clone();

        // for assign in assigns
        for assign in &ok_assigns {
            // check if the assign.guard != 0
            // should it be evaluating the guard in write_env environment?
            if eval_guard(&assign.guard, &write_env) != 0 {
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
                // XXX(karen): should read from the previous iteration's env?
                let read_val = env.get_from_port(&assign.src.borrow());

                // update internal state of the cell and
                // queue any required updates.

                //determine if dst_cell is a combinational cell or not
                if is_combinational(&dst_cell, &assign.dst.borrow().name, env) {
                    // write to assign.dst to e2 immediately, if combinational
                    write_env.put(
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

                    // now, update the internal state of the cell;
                    // for now, this only includes adds;
                    // TODO (use primitive Cell parameters)
                    let inputs;
                    let outputs;

                    // TODO: hacky way to avoid updating the cell state.
                    // Also, how to get input and output vectors in general??
                    if &assign.dst.borrow().name != "write_en" {
                        // get dst_cell's input vector
                        match &write_env.get_cell(&dst_cell) {
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
                        match &write_env.get_cell(&dst_cell) {
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
                            &dst_cell, &inputs, &outputs, &write_env,
                        )?;
                    }
                } else {
                    // otherwise, add the write to the update queue; currently only handles registers

                    // get input cell
                    let inputs = vec![src_cell.clone()];
                    // get dst_cell's output port
                    let outputs = vec![assign.dst.borrow().name.clone()];

                    write_env =
                        init_cells(&dst_cell, inputs, outputs, write_env)?;
                }
            }
        }
        // write_env = iter_updates.do_tick()
        write_env = write_env.do_tick();
        counter += 1;
    }

    /*println!(
        "\nFinal state of the done cell, i.e. {:1}: {:?} \n",
        &done_cell,
        write_env.map.get(&done_cell)
    );*/
    Ok(write_env)
}

/// Evaluate guard implementation
#[allow(clippy::borrowed_box)]
// XXX: Allow for this warning. It would make sense to use a reference when we
// have the `box` match pattern available in Rust.
fn eval_guard(guard: &Box<ir::Guard>, env: &Environment) -> u64 {
    (match &**guard {
        ir::Guard::Or(g1, g2) => {
            (eval_guard(g1, env) == 1) || (eval_guard(g2, env) == 1)
        }
        ir::Guard::And(g1, g2) => {
            (eval_guard(g1, env) == 1) && (eval_guard(g2, env) == 1)
        }
        ir::Guard::Not(g) => eval_guard(g, &env) != 0,
        ir::Guard::Eq(g1, g2) => {
            env.get_from_port(&g1.borrow()) == env.get_from_port(&g2.borrow())
        }
        ir::Guard::Neq(g1, g2) => {
            env.get_from_port(&g1.borrow()) != env.get_from_port(&g2.borrow())
        }
        ir::Guard::Gt(g1, g2) => {
            env.get_from_port(&g1.borrow()) > env.get_from_port(&g2.borrow())
        }
        ir::Guard::Lt(g1, g2) => {
            env.get_from_port(&g1.borrow()) < env.get_from_port(&g2.borrow())
        }
        ir::Guard::Geq(g1, g2) => {
            env.get_from_port(&g1.borrow()) >= env.get_from_port(&g2.borrow())
        }
        ir::Guard::Leq(g1, g2) => {
            env.get_from_port(&g1.borrow()) <= env.get_from_port(&g2.borrow())
        }
        ir::Guard::Port(p) => env.get_from_port(&p.borrow()) != 0,
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

/// Determines if writing a particular cell and cell port is combinational or not. Will need to change implementation later.
fn is_combinational(cell: &ir::Id, port: &ir::Id, env: &Environment) -> bool {
    // if cell is none,
    let cellg = env
        .get_cell(cell)
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
        _ => false,
    }
}

/// Initializes values for the update queue, i.e. for non-combinational cells
#[allow(clippy::unnecessary_unwrap)]
fn init_cells(
    cell: &ir::Id,
    inputs: Vec<ir::Id>,
    outputs: Vec<ir::Id>,
    mut env: Environment,
) -> FutilResult<Environment> {
    let cell_r = env
        .get_cell(cell)
        .unwrap_or_else(|| panic!("Cannot find cell with name"));

    // get the cell type
    match cell_r.borrow().type_name() {
        None => panic!("bad"),
        Some(ct) => match ct.id.as_str() {
            "std_sqrt" => { //:(
                 // has intermediate steps/computation
            }
            "std_reg" => {
                let map: HashMap<String, u64> = HashMap::new();
                // reg.in = dst port should go here
                env.add_update(cell.clone(), inputs, outputs, map);
            }
            _ => panic!(
                "attempted to initalize an update for a combinational cell"
            ),
        },
    }

    Ok(env)
}
