//! Defines update methods for the various primitive cells in the Calyx standard library.

use super::environment::Environment;
use super::values::Value;
use calyx::{errors::FutilResult, ir};
use std::collections::HashMap;
use std::convert::TryInto;
use std::ops::*;

/// A Standard Register of a certain [width]
/// Note that StdReg itself doen't have any bookkeeping related to clock cycles.
/// Nor does it prevent the user from reading a value before the [done] signal is high.
/// The only check it performs is preventing the user from writing
/// to the register while the [write_en] signal is low. Rules regarding cycle count,
/// such as asserting [done] for just one cycle after a write, must be enforced and
/// carried out by the interpreter.
pub struct StdReg {
    pub width: u64,
    pub val: Value,
    pub done: bool,
    pub write_en: bool,
}

impl StdReg {
    /// New registers have unitialized values -- only specify their widths
    pub fn new(width: u64) -> StdReg {
        StdReg {
            width,
            val: Value::new(width as usize),
            done: false,
            write_en: false,
        }
    }

    /// Sets value in register, only if [write_en] is high. Will
    /// truncate [input] if its [width] exceeds this register's [width]
    pub fn load_value(&mut self, input: Value) {
        if self.write_en {
            self.val = input.truncate(self.width.try_into().unwrap())
        }
    }

    /// Given a [u64], sets the corresponding [Value] in the register, only if [write_en] is high.
    /// Truncates [input]'s corresponding [Value] if its [width] exceeds this register's [width]
    pub fn load_u64(&mut self, input: u64) {
        if self.write_en {
            self.val = Value::from_init::<usize>(
                input.try_into().unwrap(),
                self.width.try_into().unwrap(),
            )
        }
    }

    /// After loading a value into the register, use [set_done_high] to emit the done signal.
    /// Note that the [StdReg] struct has no sense of time itself. The interpreter is responsible
    /// For setting the [done] signal high for exactly one cycle.
    pub fn set_done_high(&mut self) {
        self.done = true
    }
    /// Pairs with [set_done_high]
    pub fn set_done_low(&mut self) {
        self.done = false
    }

    /// A cycle before trying to load a value into the register, make sure to [set_write_en_high]
    pub fn set_write_en_high(&mut self) {
        self.write_en = true
    }

    pub fn set_write_en_low(&mut self) {
        self.write_en = false
    }

    /// Reads the value from the register. Makes no guarantee on the validity of data
    /// in the register -- the interpreter must check [done] itself.
    pub fn read_value(&self) -> Value {
        self.val.clone()
    }

    pub fn read_u64(&self) -> u64 {
        self.val.as_u64()
    }
}

pub struct StdConst {
    width: u64,
    val: Value,
}

impl StdConst {
    pub fn new(width: u64, val: Value) -> StdConst {
        StdConst {
            width,
            val: val.truncate(width as usize),
        }
    }
    pub fn read_val(&self) -> Value {
        self.val.clone()
    }
    pub fn read_u64(&self) -> u64 {
        self.val.as_u64()
    }
}

pub struct StdLsh {
    //doesn't need anything inside really
}

impl StdLsh {
    fn execute(mut val: Value) -> Value {
        val.vec.reverse();
        val.vec.remove(val.vec.len() - 1);
        val.vec.insert(0, false);
        val.vec.reverse();
        Value { vec: val.vec }
    }

    pub fn execute_u64(input: u64) -> u64 {
        // let val = Value::from_init(input.try_into().unwrap(), 64 as usize);
        // let val = Value {
        // }
        todo!()
    }
}

pub struct StdRsh {
    //doesn't need anything inside really
}

impl StdRsh {
    pub fn execute(mut val: Value) -> Value {
        val.vec.remove(val.vec.len() - 1);
        val.vec.insert(0, false);
        Value { vec: val.vec }
    }

    pub fn execute_u64(input: u64) -> u64 {
        // let val = Value::from_init(input, 64 as usize);
        todo!()
    }
}

pub struct StdAdd {
    //doesn't need anything inside really
}

impl StdAdd {
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

pub struct StdSub {
    //doesn't need anything inside really
}

impl StdSub {
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

pub struct StdSlice {
    //doesn't need anything inside really
}

impl StdSlice {
    fn execute(val: Value, width: usize) -> Value {
        val.truncate(width)
    }
}

pub struct StdPad {
    //doesn't need anything inside really
}

impl StdPad {
    fn execute(val: Value, width: usize) -> Value {
        val.ext(width)
    }
}

/// Logical Operators - TODO: need to verify
pub struct StdNot {
    //doesn't need anything inside really
}

impl StdNot {
    fn execute(val: Value) -> Value {
        Value { vec: val.vec.not() }
    }
}

pub struct StdAnd {
    //doesn't need anything inside really
}

impl StdAnd {
    fn execute(left: Value, right: Value) -> Value {
        Value {
            vec: left.vec.bitand(right.vec),
        }
    }
}

pub struct StdOr {
    //doesn't need anything inside really
}

impl StdOr {
    fn execute(left: Value, right: Value) -> Value {
        Value {
            vec: left.vec.bitor(right.vec),
        }
    }
}

pub struct StdXor {
    //doesn't need anything inside really
}

impl StdXor {
    fn execute(left: Value, right: Value) -> Value {
        Value {
            vec: left.vec.bitxor(right.vec),
        }
    }
}

/// Comparison Operators
// pub trait Execute {
//     fn execute(left: Value, right: Value, F: Fn(u64, u64) -> u64) -> Value {
//         let left_64 = left.as_u64();
//         let right_64 = right.as_u64();
//         let init_val_usize: usize = init_val.try_into().unwrap();
//         let bitwidth: usize = left.vec.len();
//         Value::from_init(init_val_usize, bitwidth)
//     }
// }

pub struct StdGt {
    //doesn't need anything inside really
}

impl StdGt {
    fn execute(left: Value, right: Value) -> Value {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 > right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val_usize, bitwidth)
    }
}

pub struct StdLt {
    //doesn't need anything inside really
}

impl StdLt {
    fn execute(left: Value, right: Value) -> Value {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 < right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val_usize, bitwidth)
    }
}

pub struct StdEq {
    //doesn't need anything inside really
}

impl StdEq {
    fn execute(left: Value, right: Value) -> Value {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 == right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val_usize, bitwidth)
    }
}

pub struct StdNeq {
    //doesn't need anything inside really
}

impl StdNeq {
    fn execute(left: Value, right: Value) -> Value {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 != right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val_usize, bitwidth)
    }
}

pub struct StdGe {
    //doesn't need anything inside really
}

impl StdGe {
    fn execute(left: Value, right: Value) -> Value {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 >= right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val_usize, bitwidth)
    }
}

pub struct StdLe {
    //doesn't need anything inside really
}

impl StdLe {
    fn execute(left: Value, right: Value) -> Value {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 <= right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val_usize, bitwidth)
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
