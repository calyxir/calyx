//! Defines update methods for the various primitive cells in the Calyx standard library.

use super::environment::Environment;
use super::values::Value;
use calyx::{errors::FutilResult, ir};
use std::collections::HashMap;
use std::convert::TryInto;
use std::ops::*;

pub trait ExecuteBinary {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value;
}

pub trait Execute {
    fn execute<'a>(
        &self,
        inputs: &'a [(ir::Id, Value)],
        outputs: &'a [(ir::Id, Value)],
    ) -> Vec<(ir::Id, Value)>;
}

pub trait ExecuteUnary {
    fn execute_unary(&self, input: &Value) -> Value;
}

impl<T: ExecuteBinary> Execute for T {
    fn execute<'a>(
        &self,
        inputs: &'a [(ir::Id, Value)],
        _outputs: &'a [(ir::Id, Value)],
    ) -> Vec<(ir::Id, Value)> {
        let (_, left) = inputs.iter().find(|(id, _)| id == "left").unwrap();

        let (_, right) = inputs.iter().find(|(id, _)| id == "right").unwrap();

        let out = T::execute_bin(self, left, right);
        vec![(ir::Id::from("out"), out)]
    }
}

/// Ensures the input values are of the appropriate widths, else panics
fn check_widths(left: &Value, right: &Value, width: u64) -> () {
    // checks len left == len right == width
    if width != (left.vec.len() as u64)
        || width != (right.vec.len() as u64)
        || left.vec.len() != right.vec.len()
    {
        panic!("Width mismatch between the component and the value.");
    }
}

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
        check_widths(&input, &input, self.width);
        if self.write_en {
            self.val = input.truncate(self.width.try_into().unwrap())
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

///A component that keeps one value, that can't be rewritten. Is instantiated with the
///value
impl StdConst {
    pub fn new(width: u64, val: Value) -> StdConst {
        StdConst {
            width,
            val: val.truncate(width as usize),
        }
    }

    pub fn new_from_u64(width: u64, val: u64) -> StdConst {
        StdConst {
            width,
            val: Value::from_init::<usize, usize>(
                val.try_into().unwrap(),
                width.try_into().unwrap(),
            ),
        }
    }

    pub fn read_val(&self) -> Value {
        self.val.clone()
    }
    pub fn read_u64(&self) -> u64 {
        self.val.as_u64()
    }
}

//NOTE: This is implemented incorrectly -- actually needs to take in two inputs. See
//documentation ( a left input is value to be shifted, right is shift amount )
pub struct StdLsh {
    width: u64,
}

impl StdLsh {
    pub fn new(width: u64) -> StdLsh {
        StdLsh { width }
    }
}

impl ExecuteBinary for StdLsh {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        //check for width -- if inapropriate, panic!
        check_widths(left, right, self.width);
        let mut tr = left.vec.clone();
        tr.shift_right(right.as_u64() as usize);
        Value { vec: tr }
    }
}

//NOTE: This is implemented incorrectly -- actually needs to take in two inputs. See
//documentation ( a left input is value to be shifted, right is shift amount )
pub struct StdRsh {
    width: u64,
}

impl StdRsh {
    pub fn new(width: u64) -> StdRsh {
        StdRsh { width }
    }
}

impl ExecuteBinary for StdRsh {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let mut tr = left.vec.clone();
        tr.shift_left(right.as_u64() as usize);
        Value { vec: tr }
    }
}

pub struct StdAdd {
    width: u64,
}

impl StdAdd {
    pub fn new(width: u64) -> StdAdd {
        StdAdd { width }
    }
}

impl ExecuteBinary for StdAdd {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        //the below will check they are all the same width so no need
        //to check left and right have same width
        check_widths(left, right, self.width);

        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 + right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val_usize, bitwidth)
    }
}

pub struct StdSub {
    width: u64,
}

impl StdSub {
    pub fn new(width: u64) -> StdSub {
        StdSub { width }
    }
}

impl ExecuteBinary for StdSub {
    //have to add width check here
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 - right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val_usize, bitwidth)
    }
}

///std_slice<IN_WIDTH, OUT_WIDTH>
pub struct StdSlice {
    in_width: u64,
    out_width: u64,
}

///Slice out the lower OUT_WIDTH bits of an IN_WIDTH-bit value. Computes in[out_width - 1 : 0]. This component is combinational.
// Inputs:

// in: IN_WIDTH - An IN_WIDTH-bit value
// Outputs:

// out: OUT_WIDTH - The lower OUT_WIDTH bits of in

impl StdSlice {
    pub fn new(in_width: u64, out_width: u64) -> StdSlice {
        StdSlice {
            in_width,
            out_width,
        }
    }
}

impl ExecuteUnary for StdSlice {
    fn execute_unary(&self, input: &Value) -> Value {
        check_widths(input, input, self.in_width);
        let tr = input.clone();
        tr.truncate(self.out_width as usize)
    }
}

pub struct StdPad {
    in_width: u64,
    out_width: u64,
}

impl StdPad {
    pub fn new(in_width: u64, out_width: u64) -> StdPad {
        StdPad {
            in_width,
            out_width,
        }
    }
}

impl ExecuteUnary for StdPad {
    fn execute_unary(&self, input: &Value) -> Value {
        check_widths(input, input, self.in_width);
        let pd = input.clone();
        pd.ext(self.out_width as usize)
    }
}

/// Logical Operators
pub struct StdNot {
    width: u64,
}

impl StdNot {
    pub fn new(width: u64) -> StdNot {
        StdNot { width }
    }
}

impl ExecuteUnary for StdNot {
    fn execute_unary<'a>(&self, input: &Value) -> Value {
        check_widths(input, input, self.width);
        Value {
            vec: input.vec.clone().not(),
        }
    }
}

pub struct StdAnd {
    width: u64,
}

impl StdAnd {
    pub fn new(width: u64) -> StdAnd {
        StdAnd { width }
    }
}

impl ExecuteBinary for StdAnd {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        Value {
            vec: left.vec.clone() & right.vec.clone(),
        }
    }
}

pub struct StdOr {
    width: u64,
}

impl StdOr {
    pub fn new(width: u64) -> StdOr {
        StdOr { width }
    }
}

impl ExecuteBinary for StdOr {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        Value {
            vec: left.vec.clone() | right.vec.clone(),
        }
    }
}

pub struct StdXor {
    width: u64,
}

impl StdXor {
    pub fn new(width: u64) -> StdXor {
        StdXor { width }
    }
}

impl ExecuteBinary for StdXor {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        Value {
            vec: left.vec.clone() ^ right.vec.clone(),
        }
    }
}

/// Comparison Operators
pub struct StdGt {
    width: u64,
}

impl StdGt {
    pub fn new(width: u64) -> StdGt {
        StdGt { width }
    }
}

impl ExecuteBinary for StdGt {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 > right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        Value::from_init(init_val_usize, 1 as usize)
    }
}

pub struct StdLt {
    width: u64,
}

impl StdLt {
    pub fn new(width: u64) -> StdLt {
        StdLt { width }
    }
}

impl ExecuteBinary for StdLt {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 < right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        Value::from_init(init_val_usize, 1 as usize)
    }
}

pub struct StdEq {
    width: u64,
}

impl StdEq {
    pub fn new(width: u64) -> StdEq {
        StdEq { width }
    }
}

impl ExecuteBinary for StdEq {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 == right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        Value::from_init(init_val_usize, 1 as usize)
    }
}

pub struct StdNeq {
    width: u64,
}

impl StdNeq {
    pub fn new(width: u64) -> StdNeq {
        StdNeq { width }
    }
}

impl ExecuteBinary for StdNeq {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 != right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        Value::from_init(init_val_usize, 1 as usize)
    }
}

pub struct StdGe {
    width: u64,
}
impl StdGe {
    pub fn new(width: u64) -> StdGe {
        StdGe { width }
    }
}
impl ExecuteBinary for StdGe {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 >= right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        Value::from_init(init_val_usize, 1 as usize)
    }
}

pub struct StdLe {
    width: u64,
}

impl StdLe {
    pub fn new(width: u64) -> StdLe {
        StdLe { width }
    }
}

impl ExecuteBinary for StdLe {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 <= right_64;

        let init_val_usize: usize = init_val.try_into().unwrap();
        Value::from_init(init_val_usize, 1 as usize)
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
