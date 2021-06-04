//! Defines update methods for the various primitive cells in the Calyx standard library.

use super::environment::Environment;
use super::values::Value;
use calyx::{errors::FutilResult, ir};
use std::collections::HashMap;
use std::convert::TryInto;

/// Struct for primitives
// pub enum Prim {
//     Add,
// }

/// Numerical Operators
//have yet to do: std_reg, std_const, std_lsh, std_rsh

//Q -- how to set the done signal high for one cycle? we have
//no concept of time in here.
//maybe just make it the interpreter's problem?
pub struct std_reg {
    val: Value,
    done: bool,
}

impl std_reg {
    fn recieve(&mut self, input: Value, write_en: bool) {
        if write_en {
            self.val = input
        }
    }
    fn set_done_high(&mut self) {
        self.done = true
    }
    fn set_done_low(&mut self) {
        self.done = false
    }
    fn read(&self) -> Value {
        self.val
    }
}

pub struct std_const {
    out: Value,
}

impl std_const {
    //is it execute? how exactly do you instantiate a std_const -- so maybe there is
    //a [put_in], or something like that?

    fn load(&mut self, val: Value) {
        self.out = val
    }

    fn read(&self) -> Value {
        self.out
    }
}

pub struct std_lsh {
    //doesn't need anything inside really
}

impl std_lsh {
    fn execute(val: Value) -> Value {
        todo!();
    }
}

pub struct std_rsh {
    //doesn't need anything inside really
}

impl std_rsh {
    fn execute(val: Value) -> Value {
        todo!();
    }
}

pub struct std_add {
    //doesn't need anything inside really
}

impl std_add {
    fn execute(left: Value, right: Value) -> Value {
        // error if left and right are different widths
        //find a bitwidth to give from
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 + right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val_usize, bitwidth)
    }
}

pub struct std_sub {
    //doesn't need anything inside really
}

impl std_sub {
    fn execute(left: Value, right: Value) -> Value {
        //find a bitwidth to give from
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 - right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val_usize, bitwidth)
    }
}

pub struct std_slice {
    //doesn't need anything inside really
}

impl std_slice {
    fn execute(val: Value, width: usize) -> Value {
        // Truncate the value
        val.truncate(width);
    }
}

pub struct std_pad {
    //doesn't need anything inside really
}

impl std_pad {
    fn execute(val: Value, width: usize) -> Value {
        // Truncate the value
        val.ext(width);
    }
}

/// Logical Operators - TODO: need to verify
pub struct std_not {
    //doesn't need anything inside really
}

impl std_not {
    fn execute(val: Value) -> Value {
        todo!();
    }
}

pub struct std_and {
    //doesn't need anything inside really
}

impl std_and {
    fn execute(left: Value, right: Value) -> Value {
        todo!();
    }
}

pub struct std_or {
    //doesn't need anything inside really
}

impl std_or {
    fn execute(left: Value, right: Value) -> Value {
        todo!();
    }
}

pub struct std_xor {
    //doesn't need anything inside really
}

impl std_xor {
    fn execute(left: Value, right: Value) -> Value {
        todo!();
    }
}

/// Comparison Operators
pub struct std_gt {
    //doesn't need anything inside really
}

impl std_gt {
    fn execute(left: Value, right: Value) -> Value {
        todo!();
    }
}

pub struct std_lt {
    //doesn't need anything inside really
}

impl std_lt {
    fn execute(left: Value, right: Value) -> Value {
        todo!();
    }
}

pub struct std_eq {
    //doesn't need anything inside really
}

impl std_eq {
    fn execute(left: Value, right: Value) -> Value {
        todo!();
    }
}

pub struct std_neq {
    //doesn't need anything inside really
}

impl std_neq {
    fn execute(left: Value, right: Value) -> Value {
        todo!();
    }
}

pub struct std_ge {
    //doesn't need anything inside really
}

impl std_ge {
    fn execute(left: Value, right: Value) -> Value {
        todo!();
    }
}

pub struct std_le {
    //doesn't need anything inside really
}

impl std_le {
    fn execute(left: Value, right: Value) -> Value {
        todo!();
    }
}

/// Uses the cell's inputs ports to perform any required updates to the
/// cell's output ports.
/// TODO: how to get input and output ports in general? How to "standardize" for combinational or not operations
pub fn update_cell_state(
    cell: &ir::Id,
    inputs: &[ir::Id],
    output: &[ir::Id],
    env: &Environment, // should this be a reference
    component: ir::Id,
) -> FutilResult<Environment> {
    // get the actual cell, based on the id
    // let cell_r = cell.as_ref();

    let mut new_env = env.clone();

    let cell_r = new_env
        .get_cell(&component, cell)
        .unwrap_or_else(|| panic!("Cannot find cell with name"));

    let temp = cell_r.borrow();

    // get the cell type
    let cell_type = temp.type_name().unwrap_or_else(|| panic!("Futil Const?"));

    match cell_type.id.as_str() {
        "std_reg" => {
            // TODO: this is wrong...
            let write_en = ir::Id::from("write_en");

            // register's write_en must be high to write reg.out and reg.done
            if new_env.get(&component, &cell, &write_en) != 0 {
                let out = ir::Id::from("out"); //assuming reg.in = cell.out, always
                let inp = ir::Id::from("in"); //assuming reg.in = cell.out, always
                let done = ir::Id::from("done"); //done id

                new_env.put(
                    &component,
                    cell,
                    &output[0],
                    env.get(&component, &inputs[0], &out),
                ); //reg.in = cell.out; should this be in init?

                if output[0].id == "in" {
                    new_env.put(
                        &component,
                        cell,
                        &out,
                        new_env.get(&component, cell, &inp),
                    ); // reg.out = reg.in
                    new_env.put(&component, cell, &done, 1); // reg.done = 1'd1
                                                             //new_env.remove_update(cell); // remove from update queue
                }
            }
        }
        "std_mem_d1" => {
            let mut mem = HashMap::new();
            let out = ir::Id::from("out");
            let write_en = ir::Id::from("write_en");
            let done = ir::Id::from("done"); //done id

            // memory should write to addres
            if new_env.get(&component, &cell, &write_en) != 0 {
                let addr0 = ir::Id::from("addr0");
                let _read_data = ir::Id::from("read_data");
                let write_data = ir::Id::from("write_data");

                new_env.put(
                    &component,
                    cell,
                    &output[0],
                    env.get(&component, &inputs[0], &out),
                );

                let data = new_env.get(&component, cell, &write_data);
                mem.insert(addr0, data);
            }
            // read data
            if output[0].id == "read_data" {
                let addr0 = ir::Id::from("addr0");

                let dat = match mem.get(&addr0) {
                    Some(&num) => num,
                    _ => panic!("nothing in the memory"),
                };

                new_env.put(&component, cell, &output[0], dat);
            }
            new_env.put(&component, cell, &done, 1);
        }
        "std_sqrt" => {
            //TODO; wrong implementation
            // new_env.put(
            //     cell,
            //     &output[0],
            //     ((new_env.get(cell, &inputs[0]) as f64).sqrt()) as u64, // cast to f64 to use sqrt
            // );
        }
        "std_add" => new_env.put(
            &component,
            cell,
            &output[0],
            new_env.get(&component, cell, &inputs[0])
                + env.get(&component, cell, &inputs[1]),
        ),
        "std_sub" => new_env.put(
            &component,
            cell,
            &output[0],
            new_env.get(&component, cell, &inputs[0])
                - env.get(&component, cell, &inputs[1]),
        ),
        "std_mod" => {
            if env.get(&component, cell, &inputs[1]) != 0 {
                new_env.put(
                    &component,
                    cell,
                    &output[0],
                    new_env.get(&component, cell, &inputs[0])
                        % env.get(&component, cell, &inputs[1]),
                )
            }
        }
        "std_mult" => new_env.put(
            &component,
            cell,
            &output[0],
            new_env.get(&component, cell, &inputs[0])
                * env.get(&component, cell, &inputs[1]),
        ),
        "std_div" => {
            // need this condition to avoid divide by 0
            // (e.g. if only one of left/right ports has been updated from the initial nonzero value?)
            // TODO: what if the program specifies a divide by 0? how to catch??
            if env.get(&component, cell, &inputs[1]) != 0 {
                new_env.put(
                    &component,
                    cell,
                    &output[0],
                    new_env.get(&component, cell, &inputs[0])
                        / env.get(&component, cell, &inputs[1]),
                )
            }
        }
        "std_not" => new_env.put(
            &component,
            cell,
            &output[0],
            !new_env.get(&component, cell, &inputs[0]),
        ),
        "std_and" => new_env.put(
            &component,
            cell,
            &output[0],
            new_env.get(&component, cell, &inputs[0])
                & env.get(&component, cell, &inputs[1]),
        ),
        "std_or" => new_env.put(
            &component,
            cell,
            &output[0],
            new_env.get(&component, cell, &inputs[0])
                | env.get(&component, cell, &inputs[1]),
        ),
        "std_xor" => new_env.put(
            &component,
            cell,
            &output[0],
            new_env.get(&component, cell, &inputs[0])
                ^ env.get(&component, cell, &inputs[1]),
        ),
        "std_gt" => new_env.put(
            &component,
            cell,
            &output[0],
            (new_env.get(&component, cell, &inputs[0])
                > env.get(&component, cell, &inputs[1])) as u64,
        ),
        "std_lt" => new_env.put(
            &component,
            cell,
            &output[0],
            (new_env.get(&component, cell, &inputs[0])
                < env.get(&component, cell, &inputs[1])) as u64,
        ),
        "std_eq" => new_env.put(
            &component,
            cell,
            &output[0],
            (new_env.get(&component, cell, &inputs[0])
                == env.get(&component, cell, &inputs[1])) as u64,
        ),
        "std_neq" => new_env.put(
            &component,
            cell,
            &output[0],
            (new_env.get(&component, cell, &inputs[0])
                != env.get(&component, cell, &inputs[1])) as u64,
        ),
        "std_ge" => new_env.put(
            &component,
            cell,
            &output[0],
            (new_env.get(&component, cell, &inputs[0])
                >= env.get(&component, cell, &inputs[1])) as u64,
        ),
        "std_le" => new_env.put(
            &component,
            cell,
            &output[0],
            (new_env.get(&component, cell, &inputs[0])
                <= env.get(&component, cell, &inputs[1])) as u64,
        ),
        _ => unimplemented!("{}", cell_type),
    }

    // TODO
    Ok(new_env)
}
