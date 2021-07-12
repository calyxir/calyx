//! Defines update methods for the various primitive cells in the Calyx
// standard library.
use super::values::{OutputValue, PulseValue, TimeLockedValue, Value};
use bitvec::prelude::*;
use calyx::ir;
use itertools::Itertools;
use serde::Serialize;
use std::ops::*;

#[derive(Clone, Debug)]
pub enum Primitive {
    StdSgt(StdSgt),
    StdAdd(StdAdd),
    StdReg(StdReg),
    StdConst(StdConst),
    StdLsh(StdLsh),
    StdRsh(StdRsh),
    StdSub(StdSub),
    StdSlice(StdSlice),
    StdPad(StdPad),
    StdNot(StdNot),
    StdAnd(StdAnd),
    StdOr(StdOr),
    StdXor(StdXor),
    StdGe(StdGe),
    StdGt(StdGt),
    StdEq(StdEq),
    StdNeq(StdNeq),
    StdLe(StdLe),
    StdLt(StdLt),
    StdMemD1(StdMemD1),
    StdMemD2(StdMemD2),
    StdMemD3(StdMemD3),
    StdMemD4(StdMemD4),
    StdMultPipe(StdMultPipe),
}

impl Primitive {
    pub fn exec_mut(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        current_done_val: Option<&Value>,
    ) -> Vec<(ir::Id, OutputValue)> {
        match self {
            Primitive::StdSgt(prim) => prim.validate_and_execute(inputs),
            Primitive::StdAdd(prim) => prim.validate_and_execute(inputs),
            Primitive::StdLsh(prim) => prim.validate_and_execute(inputs),
            Primitive::StdRsh(prim) => prim.validate_and_execute(inputs),
            Primitive::StdSub(prim) => prim.validate_and_execute(inputs),
            Primitive::StdSlice(prim) => prim.validate_and_execute(inputs),
            Primitive::StdPad(prim) => prim.validate_and_execute(inputs),
            Primitive::StdNot(prim) => prim.validate_and_execute(inputs),
            Primitive::StdAnd(prim) => prim.validate_and_execute(inputs),
            Primitive::StdOr(prim) => prim.validate_and_execute(inputs),
            Primitive::StdXor(prim) => prim.validate_and_execute(inputs),
            Primitive::StdGe(prim) => prim.validate_and_execute(inputs),
            Primitive::StdGt(prim) => prim.validate_and_execute(inputs),
            Primitive::StdEq(prim) => prim.validate_and_execute(inputs),
            Primitive::StdNeq(prim) => prim.validate_and_execute(inputs),
            Primitive::StdLe(prim) => prim.validate_and_execute(inputs),
            Primitive::StdLt(prim) => prim.validate_and_execute(inputs),
            Primitive::StdReg(prim) => {
                prim.validate_and_execute_mut(inputs, current_done_val.unwrap())
            }
            Primitive::StdMemD1(prim) => {
                prim.validate_and_execute_mut(inputs, current_done_val.unwrap())
            }
            Primitive::StdMemD2(prim) => {
                prim.validate_and_execute_mut(inputs, current_done_val.unwrap())
            }
            Primitive::StdMemD3(prim) => {
                prim.validate_and_execute_mut(inputs, current_done_val.unwrap())
            }
            Primitive::StdMemD4(prim) => {
                prim.validate_and_execute_mut(inputs, current_done_val.unwrap())
            }
            Primitive::StdMultPipe(prim) => {
                prim.validate_and_execute_mut(inputs, current_done_val.unwrap())
            }
            _ => panic!("cell cannot be executed"),
        }
    }

    pub fn reset(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        match self {
            Primitive::StdSgt(prim) => prim.validate_and_reset(inputs),
            Primitive::StdAdd(prim) => prim.validate_and_reset(inputs),
            Primitive::StdLsh(prim) => prim.validate_and_reset(inputs),
            Primitive::StdRsh(prim) => prim.validate_and_reset(inputs),
            Primitive::StdSub(prim) => prim.validate_and_reset(inputs),
            Primitive::StdSlice(prim) => prim.validate_and_reset(inputs),
            Primitive::StdPad(prim) => prim.validate_and_reset(inputs),
            Primitive::StdNot(prim) => prim.validate_and_reset(inputs),
            Primitive::StdAnd(prim) => prim.validate_and_reset(inputs),
            Primitive::StdOr(prim) => prim.validate_and_reset(inputs),
            Primitive::StdXor(prim) => prim.validate_and_reset(inputs),
            Primitive::StdGe(prim) => prim.validate_and_reset(inputs),
            Primitive::StdGt(prim) => prim.validate_and_reset(inputs),
            Primitive::StdEq(prim) => prim.validate_and_reset(inputs),
            Primitive::StdNeq(prim) => prim.validate_and_reset(inputs),
            Primitive::StdLe(prim) => prim.validate_and_reset(inputs),
            Primitive::StdLt(prim) => prim.validate_and_reset(inputs),
            Primitive::StdReg(prim) => prim.validate_and_reset(inputs),
            Primitive::StdMemD1(prim) => prim.validate_and_reset(inputs),
            Primitive::StdMemD2(prim) => prim.validate_and_reset(inputs),
            Primitive::StdMemD3(prim) => prim.validate_and_reset(inputs),
            Primitive::StdMemD4(prim) => prim.validate_and_reset(inputs),
            Primitive::StdMultPipe(prim) => prim.validate_and_reset(inputs),
            _ => panic!("cell cannot be executed"),
        }
    }

    pub fn is_comb(&self) -> bool {
        match self {
            Primitive::StdAdd(_)
            | Primitive::StdSgt(_)
            | Primitive::StdConst(_)
            | Primitive::StdLsh(_)
            | Primitive::StdRsh(_)
            | Primitive::StdSub(_)
            | Primitive::StdSlice(_)
            | Primitive::StdPad(_)
            | Primitive::StdNot(_)
            | Primitive::StdAnd(_)
            | Primitive::StdOr(_)
            | Primitive::StdXor(_)
            | Primitive::StdGe(_)
            | Primitive::StdGt(_)
            | Primitive::StdEq(_)
            | Primitive::StdNeq(_)
            | Primitive::StdLe(_)
            | Primitive::StdLt(_) => true,
            Primitive::StdMemD1(_)
            | Primitive::StdMultPipe(_)
            | Primitive::StdMemD2(_)
            | Primitive::StdMemD3(_)
            | Primitive::StdMemD4(_)
            | Primitive::StdReg(_) => false,
        }
    }

    pub fn commit_updates(&mut self) {
        match self {
            Primitive::StdAdd(_)
            | Primitive::StdSgt(_)
            | Primitive::StdConst(_)
            | Primitive::StdLsh(_)
            | Primitive::StdRsh(_)
            | Primitive::StdSub(_)
            | Primitive::StdSlice(_)
            | Primitive::StdPad(_)
            | Primitive::StdNot(_)
            | Primitive::StdAnd(_)
            | Primitive::StdOr(_)
            | Primitive::StdXor(_)
            | Primitive::StdGe(_)
            | Primitive::StdGt(_)
            | Primitive::StdEq(_)
            | Primitive::StdNeq(_)
            | Primitive::StdLe(_)
            | Primitive::StdLt(_) => {}
            Primitive::StdMultPipe(mp) => mp.commit_updates(),
            Primitive::StdReg(reg) => reg.commit_updates(),
            Primitive::StdMemD1(mem) => mem.commit_updates(),
            Primitive::StdMemD2(mem) => mem.commit_updates(),
            Primitive::StdMemD3(mem) => mem.commit_updates(),
            Primitive::StdMemD4(mem) => mem.commit_updates(),
        }
    }

    pub fn clear_update_buffer(&mut self) {
        match self {
            Primitive::StdAdd(_)
            | Primitive::StdSgt(_)
            | Primitive::StdConst(_)
            | Primitive::StdLsh(_)
            | Primitive::StdRsh(_)
            | Primitive::StdSub(_)
            | Primitive::StdSlice(_)
            | Primitive::StdPad(_)
            | Primitive::StdNot(_)
            | Primitive::StdAnd(_)
            | Primitive::StdOr(_)
            | Primitive::StdXor(_)
            | Primitive::StdGe(_)
            | Primitive::StdGt(_)
            | Primitive::StdEq(_)
            | Primitive::StdNeq(_)
            | Primitive::StdLe(_)
            | Primitive::StdLt(_) => {}
            Primitive::StdMultPipe(mp) => mp.clear_update_buffer(),
            Primitive::StdReg(reg) => reg.clear_update_buffer(),
            Primitive::StdMemD1(mem) => mem.clear_update_buffer(),
            Primitive::StdMemD2(mem) => mem.clear_update_buffer(),
            Primitive::StdMemD3(mem) => mem.clear_update_buffer(),
            Primitive::StdMemD4(mem) => mem.clear_update_buffer(),
        }
    }
}

impl Serialize for Primitive {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self {
            Primitive::StdMultPipe(prim) => prim.serialize(serializer),
            Primitive::StdReg(prim) => prim.serialize(serializer),
            Primitive::StdMemD1(prim) => prim.serialize(serializer),
            Primitive::StdMemD2(prim) => prim.serialize(serializer),
            Primitive::StdMemD3(prim) => prim.serialize(serializer),
            Primitive::StdMemD4(prim) => prim.serialize(serializer),
            Primitive::StdAdd(_)
            | Primitive::StdSgt(_)
            | Primitive::StdConst(_)
            | Primitive::StdLsh(_)
            | Primitive::StdRsh(_)
            | Primitive::StdSub(_)
            | Primitive::StdSlice(_)
            | Primitive::StdPad(_)
            | Primitive::StdNot(_)
            | Primitive::StdAnd(_)
            | Primitive::StdOr(_)
            | Primitive::StdXor(_)
            | Primitive::StdGe(_)
            | Primitive::StdGt(_)
            | Primitive::StdEq(_)
            | Primitive::StdNeq(_)
            | Primitive::StdLe(_)
            | Primitive::StdLt(_) => {
                panic!("Primitive {:?} is not serializable", self)
            }
        }
    }
}

pub trait ValidateInput {
    /// Verifies that all the given inputs are of the appropriate size
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]);
}

/// For unary operator components that only take in one input.
/// *Assumes that all inputs have the name "in", and will return
/// a (ir::Id, Value) with the Id as "out"*
/// # Example
/// ```
/// use interp::primitives::*;
/// use interp::values::*;
/// let std_not = StdNot::new(5); // a 5 bit not-er
/// let not_one = std_not.execute_unary(&(Value::try_from_init(1, 5).unwrap()));
/// ```
pub trait ExecuteUnary: ValidateInput {
    fn execute_unary(&self, input: &Value) -> OutputValue;

    /// Default implementation of [execute] for all unary components
    /// Unwraps inputs, then sends output based on [execute_unary]
    fn execute(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        let (_, input) = inputs.iter().find(|(id, _)| id == "in").unwrap();
        vec![(ir::Id::from("out"), self.execute_unary(input))]
    }

    fn reset(&self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, OutputValue)> {
        self.execute(inputs)
    }

    /// A wrapper function which invokes validate_input before proceeding with
    /// execution. Preferred over execute.
    fn validate_and_execute(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        self.validate_input(inputs);
        self.execute(inputs)
    }

    /// A wrapper function which invokes validate_input before proceeding with
    /// the reset. Preferred over reset.
    fn validate_and_reset(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        self.validate_input(inputs);
        self.reset(inputs)
    }
}

/// For binary operator components that taken in a <left> Value and
/// <right> Value.
///
/// # Example
/// ```
/// use interp::primitives::*;
/// use interp::values::*;
/// let std_add = StdAdd::new(5); // A 5 bit adder
/// let one_plus_two = std_add.execute_bin(
///     &(Value::try_from_init(1, 5).unwrap()),
///     &(Value::try_from_init(2, 5).unwrap())
/// );
/// ```
pub trait ExecuteBinary {
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue;

    fn get_width(&self) -> &u64;
    /// Default implementation of [execute] for all binary components
    /// Unwraps inputs (left and right), then sends output based on [execute_bin]
    fn execute(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        let (_, left) = inputs.iter().find(|(id, _)| id == "left").unwrap();

        let (_, right) = inputs.iter().find(|(id, _)| id == "right").unwrap();

        let out = self.execute_bin(left, right);
        vec![(ir::Id::from("out"), out)]
    }

    fn reset(&self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, OutputValue)> {
        self.execute(inputs)
    }

    /// A wrapper function which invokes validate_input before proceeding with
    /// execution. Preferred over execute.
    fn validate_and_execute(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        self.validate_input(inputs);
        self.execute(inputs)
    }

    /// A wrapper function which invokes validate_input before proceeding with
    /// the reset. Preferred over reset.
    fn validate_and_reset(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        self.validate_input(inputs);
        self.reset(inputs)
    }

    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "left" => assert_eq!(v.len() as u64, *self.get_width()),
                "right" => assert_eq!(v.len() as u64, *self.get_width()),
                _ => {}
            }
        }
    }
}

impl<T: ExecuteBinary> ValidateInput for T {
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        T::validate_input(&self, inputs)
    }
}

pub trait Execute: ValidateInput {
    fn execute(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)>;

    fn reset(&self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, OutputValue)> {
        self.execute(inputs)
    }

    /// A wrapper function which invokes validate_input before proceeding with
    /// execution. Preferred over execute.
    fn validate_and_execute(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        self.validate_input(inputs);
        self.execute(inputs)
    }

    /// A wrapper function which invokes validate_input before proceeding with
    /// the reset. Preferred over reset.
    fn validate_and_reset(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        self.validate_input(inputs);
        self.reset(inputs)
    }
}

/// ExecuteStateful is a trait implemnted by primitive components such as
/// StdReg, StdMem (D1 -- D4), and StdMultPipe allowing their state to be modified.
pub trait ExecuteStateful: ValidateInput + Serialize {
    /// Use execute_mut to modify the state of a stateful component.
    /// No restrictions on exactly how the input(s) look
    fn execute_mut(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        current_done_val: &Value,
    ) -> Vec<(ir::Id, OutputValue)>;

    fn reset(&self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, OutputValue)>;

    /// A wrapper function which invokes validate_input before proceeding with
    /// execution. Preferred over execute_mut.
    fn validate_and_execute_mut(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        current_done_val: &Value,
    ) -> Vec<(ir::Id, OutputValue)> {
        self.validate_input(inputs);
        self.execute_mut(inputs, current_done_val)
    }

    /// A wrapper function which invokes validate_input before proceeding with
    /// the reset. Preferred over reset.
    fn validate_and_reset(
        &self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        self.validate_input(inputs);
        self.reset(inputs)
    }

    /// This function transfers the update held in a primitive's buffer into the
    /// state contained within the primitive itself. Until this method is
    /// invoked, the primitive's internal state will remain unchanged by
    /// execution. This is to prevent ephemeral changes due to repeated
    /// invocations
    fn commit_updates(&mut self);

    /// Resets the primitive's update buffer without commiting the held changes,
    /// effectively reverting the write and ensuring it does not occur. Use to
    /// reset stateful primitives after a group execution.
    fn clear_update_buffer(&mut self);
}

/// Ensures the input values are of the appropriate widths, else panics.
fn check_widths(left: &Value, right: &Value, width: u64) {
    if width != (left.vec.len() as u64)
        || width != (right.vec.len() as u64)
        || left.vec.len() != right.vec.len()
    {
        panic!("Width mismatch between the component and the inputs. Comp width: {}, left: {}, right: {}", width, left.vec.len(), right.vec.len());
    }
}

/// A one-dimensional memory. Initialized with
/// StdMemD1.new(WIDTH, SIZE, IDX_SIZE) where:
/// * WIDTH - Size of an individual memory slot.
/// * SIZE - Number of slots in the memory.
/// * IDX_SIZE - The width of the index given to the memory.
///
/// To write to a memory, the [write_en] must be high.
/// Inputs:
/// * addr0: IDX_SIZE - The index to be accessed or updated.
/// * write_data: WIDTH - Data to be written to the selected memory slot.
/// * write_en: 1 - One bit write enabled signal, causes the memory to write
///             write_data to the slot indexed by addr0.
///
/// Outputs:
/// * read_data: WIDTH - The value stored at addr0. This value is combinational
///              with respect to addr0.
/// * done: 1 - The done signal for the memory. This signal goes high for one
///         cycle after finishing a write to the memory.
#[derive(Clone, Debug)]
pub struct StdMemD1 {
    pub width: u64,    // size of individual piece of mem
    pub size: u64,     // # slots of mem
    pub idx_size: u64, // # bits needed to index a piece of mem
    pub data: Vec<Value>,
    update: Option<(u64, Value)>,
}

impl StdMemD1 {
    /// Instantiates a new StdMemD1 storing data of width [width], containing [size]
    /// slots for memory, accepting indecies (addr0) of width [idx_size].
    /// Note: if [idx_size] is smaller than the length of [size]'s binary representation,
    /// you will not be able to access the slots near the end of the memory.
    pub fn new(width: u64, size: u64, idx_size: u64) -> StdMemD1 {
        let data = vec![Value::zeroes(width as usize); size as usize];
        StdMemD1 {
            width,
            size,     //how many slots of memory in the vector
            idx_size, //the width of the values used to address the memory
            data,
            update: None,
        }
    }

    pub fn initialize_memory(&mut self, vals: &[Value]) {
        assert_eq!(self.size as usize, vals.len());

        for (idx, val) in vals.iter().enumerate() {
            assert_eq!(val.len(), self.width as usize);
            self.data[idx] = val.clone()
        }
    }
}

impl ValidateInput for StdMemD1 {
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "write_data" => assert_eq!(v.len() as u64, self.width),
                "write_en" => assert_eq!(v.len(), 1),
                "addr0" => {
                    assert!(v.as_u64() < self.size);
                    assert_eq!(v.len() as u64, self.idx_size)
                }
                _ => {}
            }
        }
    }
}

impl ExecuteStateful for StdMemD1 {
    /// Takes a slice of tupules. This slice must contain the tupules ("write_data", Value),
    /// ("write_en", Value), and ("addr0", Value). Attempts to write the [write_data] Value to
    /// the memory at slot [addr0] if [write_en] is 1. Else does not modify the memory.
    /// Returns a vector of outputs in this *guaranteed* order <("read_data", OutputValue), ("done", OutputValue)>,
    /// The OutputValues are both LockedValues if [write_en] is 1. If [write_en] is
    /// not 1 then the OutputValues are ImmediateValues, [done] being 0 and
    /// [read_data] being whatever was stored in the memory at address [addr0]
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// use calyx::ir;
    ///
    /// let mut std_memd1 = StdMemD1::new(1, 8, 3); //1-bit pieces of data, 8 pieces, need 3 bits to index the memory
    /// let write_data = (ir::Id::from("write_data"), &Value::try_from_init(1, 1).unwrap());
    /// let write_en = (ir::Id::from("write_en"), &Value::try_from_init(1, 1).unwrap());
    /// let addr0 = (ir::Id::from("addr0"), &Value::try_from_init(0, 3).unwrap());
    /// let output_vals = std_memd1.execute_mut(&[write_data, write_en, addr0], &Value::bit_low());
    /// let mut output_vals = output_vals.into_iter();
    /// let (read_data, done) = (output_vals.next().unwrap(), output_vals.next().unwrap());
    /// let mut rd = read_data.1.unwrap_tlv();
    /// if let OutputValue::PulseValue(mut d) = done.1 {
    ///     assert_eq!(d.get_val().as_u64(), 0);
    ///     d.tick();
    ///     assert_eq!(d.get_val().as_u64(), 1);
    ///     d.tick();
    ///     assert_eq!(d.get_val().as_u64(), 0);
    /// } else {
    ///     panic!()
    /// }
    /// assert_eq!(rd.get_count(), 1);
    /// rd.dec_count();
    /// assert!(rd.unlockable());
    /// assert_eq!(rd.clone().unlock().as_u64(), Value::try_from_init(1, 1).unwrap().as_u64());
    /// ```
    /// # Panics
    /// Panics if [write_data] is not the same width as self.width. Panics if the width of addr0 does not equal
    /// self.idx_size.
    fn execute_mut(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        current_done_val: &Value,
    ) -> Vec<(ir::Id, OutputValue)> {
        //unwrap the arguments
        //these come from the primitive definition in verilog
        //don't need to depend on the user -- just make sure this matches primitive + calyx + verilog defs
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        //so we don't have to keep using .as_u64()
        let addr0 = addr0.as_u64();

        let old = self.data[addr0 as usize].clone();
        // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.update = Some((addr0, (*input).clone()));

            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new((*input).clone(), 1, Some(old)).into(),
                ),
                (
                    "done".into(),
                    PulseValue::new(
                        current_done_val.clone(),
                        Value::bit_high(),
                        Value::bit_low(),
                        1,
                    )
                    .into(),
                ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // READ_DATA: (immediate), just the old value b/c write was unsuccessful
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![
                (ir::Id::from("read_data"), old.into()),
                // (ir::Id::from("done"), Value::zeroes(1).into()),
            ]
        }
    }

    fn reset(&self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, OutputValue)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        //so we don't have to keep using .as_u64()
        let addr0 = addr0.as_u64();
        //check that input data is the appropriate width as well
        let old = self.data[addr0 as usize].clone();
        vec![
            ("read_data".into(), old.into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
        ]
    }

    fn commit_updates(&mut self) {
        if let Some((idx, val)) = self.update.take() {
            self.data[idx as usize] = val;
        }
    }

    fn clear_update_buffer(&mut self) {
        self.update = None;
    }
}

impl Serialize for StdMemD1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mem = self.data.iter().map(|x| x.as_u64()).collect::<Vec<_>>();
        mem.serialize(serializer)
    }
}

///std_memd2 :
/// A two-dimensional memory.
/// Parameters:
/// WIDTH - Size of an individual memory slot.
/// D0_SIZE - Number of memory slots for the first index.
/// D1_SIZE - Number of memory slots for the second index.
/// D0_IDX_SIZE - The width of the first index.
/// D1_IDX_SIZE - The width of the second index.
/// Inputs:
/// addr0: D0_IDX_SIZE - The first index into the memory
/// addr1: D1_IDX_SIZE - The second index into the memory
/// write_data: WIDTH - Data to be written to the selected memory slot
/// write_en: 1 - One bit write enabled signal, causes the memory to write write_data to the slot indexed by addr0 and addr1
/// Outputs:
/// read_data: WIDTH - The value stored at mem[addr0][addr1]. This value is combinational with respect to addr0 and addr1.
/// done: 1: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.
#[derive(Clone, Debug)]
pub struct StdMemD2 {
    pub width: u64,   // size of individual piece of mem
    pub d0_size: u64, // # slots of mem
    pub d1_size: u64,
    pub d0_idx_size: u64,
    pub d1_idx_size: u64, // # bits needed to index a piece of mem
    pub data: Vec<Value>,
    update: Option<(u64, u64, Value)>,
}

impl StdMemD2 {
    /// Instantiates a new StdMemD2 storing data of width [width], containing
    /// [d0_size] * [d1_size] slots for memory, accepting indecies [addr0][addr1] of widths
    /// [d0_idx_size] and [d1_idx_size] respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(
        width: u64,
        d0_size: u64,
        d1_size: u64,
        d0_idx_size: u64,
        d1_idx_size: u64,
    ) -> StdMemD2 {
        //data is a 2d vector
        let data =
            vec![Value::zeroes(width as usize); (d0_size * d1_size) as usize];
        StdMemD2 {
            width,
            d0_size,
            d1_size,
            d0_idx_size,
            d1_idx_size,
            data,
            update: None,
        }
    }

    pub fn initialize_memory(&mut self, vals: &[Value]) {
        assert_eq!((self.d0_size * self.d1_size) as usize, vals.len());

        for (idx, val) in vals.iter().enumerate() {
            assert_eq!(val.len(), self.width as usize);
            self.data[idx] = val.clone()
        }
    }

    #[inline]
    fn calc_addr(&self, addr0: u64, addr1: u64) -> u64 {
        addr0 * self.d1_size + addr1
    }
}

impl ValidateInput for StdMemD2 {
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "write_data" => assert_eq!(v.len() as u64, self.width),
                "write_en" => assert_eq!(v.len(), 1),
                "addr0" => {
                    assert!(v.as_u64() < self.d0_size);
                    assert_eq!(v.len() as u64, self.d0_idx_size)
                }
                "addr1" => {
                    assert!(v.as_u64() < self.d1_size);
                    assert_eq!(v.len() as u64, self.d1_idx_size)
                }
                _ => {}
            }
        }
    }
}

impl ExecuteStateful for StdMemD2 {
    /// Takes a slice of tupules. This slice must contain the tupules ("write_data", Value),
    /// ("write_en", Value), ("addr0", Value), and ("addr1", Value). Attempts to write the [write_data] Value to
    /// the memory at slot [addr0][addr1] if [write_en] is 1. Else does not modify the memory.
    /// Returns a vector of outputs in this *guaranteed* order <("read_data", OutputValue), ("done", OutputValue)>,
    /// The OutputValues are both LockedValues if [write_en] is 1. If [write_en] is
    /// not 1 then the OutputValues are ImmediateValues, [done] being 0 and
    /// [read_data] being whatever was stored in the memory at address [addr0][addr1]
    /// # Example
    /// See example in StdMemD1
    /// # Panics
    /// Panics if [write_data] is not the same width as self.width. Panics if the width of addr0 does not equal
    /// self.d0_idx_size. Panics if the width of addr1 does not equal self.d1_idx_size.
    fn execute_mut(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        current_done_val: &Value,
    ) -> Vec<(ir::Id, OutputValue)> {
        //unwrap the arguments
        //these come from the primitive definition in verilog
        //don't need to depend on the user -- just make sure this matches primitive + calyx + verilog defs
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        //chec that write_data is the exact right width
        //check that addr0 is not out of bounds and that it is the proper width!
        let addr0 = addr0.as_u64();

        //chech that addr1 is not out of bounds and that it is the proper iwdth
        let addr1 = addr1.as_u64();
        let real_addr = self.calc_addr(addr0, addr1);

        let old = self.data[real_addr as usize].clone(); //not sure if this could lead to errors (Some(old)) is borrow?
                                                         // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.update = Some((addr0, addr1, (*input).clone()));
            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new((*input).clone(), 1, Some(old)).into(),
                ),
                (
                    "done".into(),
                    PulseValue::new(
                        current_done_val.clone(),
                        Value::bit_high(),
                        Value::bit_low(),
                        1,
                    )
                    .into(),
                ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // READ_DATA: (immediate), just the old value b/c write was unsuccessful
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![
                (ir::Id::from("read_data"), old.into()),
                // (ir::Id::from("done"), Value::zeroes(1).into()),
            ]
        }
    }

    fn reset(&self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, OutputValue)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();

        let real_addr = self.calc_addr(addr0, addr1);

        let old = self.data[real_addr as usize].clone();

        vec![
            (ir::Id::from("read_data"), old.into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
        ]
    }

    fn commit_updates(&mut self) {
        if let Some((addr0, addr1, val)) = self.update.take() {
            let real_addr = self.calc_addr(addr0, addr1);
            self.data[real_addr as usize] = val;
        }
    }

    fn clear_update_buffer(&mut self) {
        self.update = None;
    }
}

impl Serialize for StdMemD2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mem = self
            .data
            .iter()
            .chunks(self.d1_size as usize)
            .into_iter()
            .map(|x| x.into_iter().map(|y| y.as_u64()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        mem.serialize(serializer)
    }
}

///std_memd3 :
/// A three-dimensional memory.
/// Parameters:
/// WIDTH - Size of an individual memory slot.
/// D0_SIZE - Number of memory slots for the first index.
/// D1_SIZE - Number of memory slots for the second index.
/// D2_SIZE - Number of memory slots for the third index.
/// D0_IDX_SIZE - The width of the first index.
/// D1_IDX_SIZE - The width of the second index.
/// D2_IDX_SIZE - The width of the third index.
/// Inputs:
/// addr0: D0_IDX_SIZE - The first index into the memory
/// addr1: D1_IDX_SIZE - The second index into the memory
/// addr2: D2_IDX_SIZE - The third index into the memory
/// write_data: WIDTH - Data to be written to the selected memory slot
/// write_en: 1 - One bit write enabled signal, causes the memory to write write_data to the slot indexed by addr0, addr1, and addr2
/// Outputs:
/// read_data: WIDTH - The value stored at mem[addr0][addr1][addr2]. This value is combinational with respect to addr0, addr1, and addr2.
/// done: 1: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.
#[derive(Clone, Debug)]
pub struct StdMemD3 {
    width: u64,
    d0_size: u64,
    d1_size: u64,
    d2_size: u64,
    d0_idx_size: u64,
    d1_idx_size: u64,
    d2_idx_size: u64,
    data: Vec<Value>,
    update: Option<(u64, u64, u64, Value)>,
}

impl StdMemD3 {
    /// Instantiates a new StdMemD3 storing data of width [width], containing
    /// [d0_size] * [d1_size] * [d2_size] slots for memory, accepting indecies [addr0][addr1][addr2] of widths
    /// [d0_idx_size], [d1_idx_size], and [d2_idx_size] respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(
        width: u64,
        d0_size: u64,
        d1_size: u64,
        d2_size: u64,
        d0_idx_size: u64,
        d1_idx_size: u64,
        d2_idx_size: u64,
    ) -> StdMemD3 {
        let data = vec![
            Value::zeroes(width as usize);
            (d0_size * d1_size * d2_size) as usize
        ];
        StdMemD3 {
            width,
            d0_size,
            d1_size,
            d2_size,
            d0_idx_size,
            d1_idx_size,
            d2_idx_size,
            data,
            update: None,
        }
    }

    pub fn initialize_memory(&mut self, vals: &[Value]) {
        assert_eq!(
            (self.d0_size * self.d1_size * self.d2_size) as usize,
            vals.len()
        );

        for (idx, val) in vals.iter().enumerate() {
            assert_eq!(val.len(), self.width as usize);
            self.data[idx] = val.clone()
        }
    }

    #[inline]
    fn calc_addr(&self, addr0: u64, addr1: u64, addr2: u64) -> u64 {
        self.d2_size * (addr0 * self.d1_size + addr1) + addr2
    }
}

impl ValidateInput for StdMemD3 {
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "write_data" => assert_eq!(v.len() as u64, self.width),
                "write_en" => assert_eq!(v.len(), 1),
                "addr0" => {
                    assert!(v.as_u64() < self.d0_size);
                    assert_eq!(v.len() as u64, self.d0_idx_size)
                }
                "addr1" => {
                    assert!(v.as_u64() < self.d1_size);
                    assert_eq!(v.len() as u64, self.d1_idx_size)
                }
                "addr2" => {
                    assert!(v.as_u64() < self.d2_size);
                    assert_eq!(v.len() as u64, self.d2_idx_size)
                }
                _ => {}
            }
        }
    }
}

impl ExecuteStateful for StdMemD3 {
    /// Takes a slice of tupules. This slice must contain the tupules ("write_data", Value),
    /// ("write_en", Value), ("addr0", Value), ("addr1", Value), and ("addr2", Value). Attempts to write the [write_data] Value to
    /// the memory at slot [addr0][addr1][addr2] if [write_en] is 1. Else does not modify the memory.
    /// Returns a vector of outputs in this *guaranteed* order <("read_data", OutputValue), ("done", OutputValue)>,
    /// The OutputValues are both LockedValues if [write_en] is 1. If [write_en] is
    /// not 1 then the OutputValues are ImmediateValues, [done] being 0 and
    /// [read_data] being whatever was stored in the memory at address [addr0][addr1][addr2]
    /// # Example
    /// See example in StdMemD1
    /// # Panics
    /// Panics if [write_data] is not the same width as self.width. Panics if the width of addr0 does not equal
    /// self.d0_idx_size. Panics if the width of addr1 does not equal self.d1_idx_size. Panics if the
    /// width of addr2 does not equal self.d2_idx_size.
    fn execute_mut(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        current_done_val: &Value,
    ) -> Vec<(ir::Id, OutputValue)> {
        //unwrap the arguments
        //these come from the primitive definition in verilog
        //don't need to depend on the user -- just make sure this matches primitive + calyx + verilog defs
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let (_, addr2) = inputs.iter().find(|(id, _)| id == "addr2").unwrap();

        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();

        let real_addr = self.calc_addr(addr0, addr1, addr2);

        let old = self.data[real_addr as usize].clone();
        //not sure if this could lead to errors (Some(old)) is borrow?
        // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.update = Some((addr0, addr1, addr2, (*input).clone()));

            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new((*input).clone(), 1, Some(old)).into(),
                ),
                (
                    "done".into(),
                    PulseValue::new(
                        current_done_val.clone(),
                        Value::bit_high(),
                        Value::bit_low(),
                        1,
                    )
                    .into(),
                ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // READ_DATA: (immediate), just the old value b/c write was unsuccessful
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![
                (ir::Id::from("read_data"), old.into()),
                // (ir::Id::from("done"), Value::zeroes(1).into()),
            ]
        }
    }

    fn reset(&self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, OutputValue)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let (_, addr2) = inputs.iter().find(|(id, _)| id == "addr2").unwrap();
        //check that addr0 is not out of bounds and that it is the proper width!
        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();

        let real_addr = self.calc_addr(addr0, addr1, addr2);

        let old = self.data[real_addr as usize].clone();
        vec![
            (ir::Id::from("read_data"), old.into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
        ]
    }

    fn commit_updates(&mut self) {
        if let Some((addr0, addr1, addr2, val)) = self.update.take() {
            let real_addr = self.calc_addr(addr0, addr1, addr2);

            self.data[real_addr as usize] = val;
        }
    }

    fn clear_update_buffer(&mut self) {
        self.update = None;
    }
}

impl Serialize for StdMemD3 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mem = self
            .data
            .iter()
            .chunks((self.d2_size * self.d1_size) as usize)
            .into_iter()
            .map(|x| {
                x.into_iter()
                    .chunks(self.d1_size as usize)
                    .into_iter()
                    .map(|y| {
                        y.into_iter().map(|z| z.as_u64()).collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        mem.serialize(serializer)
    }
}
///std_memd4
/// std_mem_d4
/// A four-dimensional memory.
/// Parameters:
/// WIDTH - Size of an individual memory slot.
/// D0_SIZE - Number of memory slots for the first index.
/// D1_SIZE - Number of memory slots for the second index.
/// D2_SIZE - Number of memory slots for the third index.
/// D3_SIZE - Number of memory slots for the fourth index.
/// D0_IDX_SIZE - The width of the first index.
/// D1_IDX_SIZE - The width of the second index.
/// D2_IDX_SIZE - The width of the third index.
/// D3_IDX_SIZE - The width of the fourth index.
/// Inputs:
/// addr0: D0_IDX_SIZE - The first index into the memory
/// addr1: D1_IDX_SIZE - The second index into the memory
/// addr2: D2_IDX_SIZE - The third index into the memory
/// addr3: D3_IDX_SIZE - The fourth index into the memory
/// write_data: WIDTH - Data to be written to the selected memory slot
/// write_en: 1 - One bit write enabled signal, causes the memory to write write_data to the slot indexed by addr0, addr1, addr2, and addr3
/// Outputs:
/// read_data: WIDTH - The value stored at mem[addr0][addr1][addr2][addr3]. This value is combinational with respect to addr0, addr1, addr2, and addr3.
/// done: 1: The done signal for the memory. This signal goes high for one cycle after finishing a write to the memory.
#[derive(Clone, Debug)]
pub struct StdMemD4 {
    width: u64,
    d0_size: u64,
    d1_size: u64,
    d2_size: u64,
    d3_size: u64,
    d0_idx_size: u64,
    d1_idx_size: u64,
    d2_idx_size: u64,
    d3_idx_size: u64,
    data: Vec<Value>,
    update: Option<(u64, u64, u64, u64, Value)>,
}

impl StdMemD4 {
    #[allow(clippy::too_many_arguments)]
    // Instantiates a new StdMemD3 storing data of width [width], containing
    /// [d0_size] * [d1_size] * [d2_size] * [d3_size] slots for memory, accepting indecies [addr0][addr1][addr2][addr3] of widths
    /// [d0_idx_size], [d1_idx_size], [d2_idx_size] and [d3_idx_size] respectively.
    /// Initially the memory is filled with all 0s.
    pub fn new(
        width: u64,
        d0_size: u64,
        d1_size: u64,
        d2_size: u64,
        d3_size: u64,
        d0_idx_size: u64,
        d1_idx_size: u64,
        d2_idx_size: u64,
        d3_idx_size: u64,
    ) -> StdMemD4 {
        let data = vec![
            Value::zeroes(width as usize);
            (d0_size * d1_size * d2_size * d3_size) as usize
        ];
        StdMemD4 {
            width,
            d0_size,
            d1_size,
            d2_size,
            d3_size,
            d0_idx_size,
            d1_idx_size,
            d2_idx_size,
            d3_idx_size,
            data,
            update: None,
        }
    }

    pub fn initialize_memory(&mut self, vals: &[Value]) {
        assert_eq!(
            (self.d0_size * self.d1_size * self.d2_size * self.d3_size)
                as usize,
            vals.len()
        );

        for (idx, val) in vals.iter().enumerate() {
            assert_eq!(val.len(), self.width as usize);
            self.data[idx] = val.clone()
        }
    }

    #[inline]
    fn calc_addr(&self, addr0: u64, addr1: u64, addr2: u64, addr3: u64) -> u64 {
        self.d3_size * (self.d2_size * (addr0 * self.d1_size + addr1) + addr2)
            + addr3
    }
}

impl ValidateInput for StdMemD4 {
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "write_data" => assert_eq!(v.len() as u64, self.width),
                "write_en" => assert_eq!(v.len(), 1),
                "addr0" => {
                    assert!(v.as_u64() < self.d0_size);
                    assert_eq!(v.len() as u64, self.d0_idx_size)
                }
                "addr1" => {
                    assert!(v.as_u64() < self.d1_size);
                    assert_eq!(v.len() as u64, self.d1_idx_size)
                }
                "addr2" => {
                    assert!(v.as_u64() < self.d2_size);
                    assert_eq!(v.len() as u64, self.d2_idx_size)
                }
                "addr3" => {
                    assert!(v.as_u64() < self.d3_size);
                    assert_eq!(v.len() as u64, self.d3_idx_size)
                }
                _ => {}
            }
        }
    }
}

impl ExecuteStateful for StdMemD4 {
    /// Takes a slice of tupules. This slice must contain the tupules ("write_data", Value),
    /// ("write_en", Value), ("addr0", Value), ("addr1", Value), ("addr2", Value). Attempts to write the [write_data] Value to
    /// the memory at slot [addr0][addr1][addr2] if [write_en] is 1. Else does not modify the memory.
    /// Returns a vector of outputs in this *guaranteed* order <("read_data", OutputValue), ("done", OutputValue)>,
    /// The OutputValues are both LockedValues if [write_en] is 1. If [write_en] is
    /// not 1 then the OutputValues are ImmediateValues, [done] being 0 and
    /// [read_data] being whatever was stored in the memory at address [addr0][addr1][addr2][addr3]
    /// # Example
    /// See example in StdMemD1
    /// # Panics
    /// Panics if [write_data] is not the same width as self.width. Panics if the width of addr0 does not equal
    /// self.d0_idx_size. Panics if the width of addr1 does not equal self.d1_idx_size. Panics if the
    /// width of addr2 does not equal self.d2_idx_size. Panics if the width of addr3 does not equal self.d3_idx_size.
    fn execute_mut(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        current_done_val: &Value,
    ) -> Vec<(ir::Id, OutputValue)> {
        //unwrap the arguments
        //these come from the primitive definition in verilog
        //don't need to depend on the user -- just make sure this matches primitive + calyx + verilog defs
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let (_, addr2) = inputs.iter().find(|(id, _)| id == "addr2").unwrap();
        let (_, addr3) = inputs.iter().find(|(id, _)| id == "addr3").unwrap();

        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();
        let addr3 = addr3.as_u64();

        let real_addr = self.calc_addr(addr0, addr1, addr2, addr3);

        let old = self.data[real_addr as usize].clone(); //not sure if this could lead to errors (Some(old)) is borrow?
                                                         // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.update = Some((addr0, addr1, addr2, addr3, (*input).clone()));

            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new((*input).clone(), 1, Some(old)).into(),
                ),
                (
                    "done".into(),
                    PulseValue::new(
                        current_done_val.clone(),
                        Value::bit_high(),
                        Value::bit_low(),
                        1,
                    )
                    .into(),
                ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // READ_DATA: (immediate), just the old value b/c write was unsuccessful
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![
                (ir::Id::from("read_data"), old.into()),
                // (ir::Id::from("done"), Value::zeroes(1).into()),
            ]
        }
    }

    fn reset(&self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, OutputValue)> {
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        let (_, addr1) = inputs.iter().find(|(id, _)| id == "addr1").unwrap();
        let (_, addr2) = inputs.iter().find(|(id, _)| id == "addr2").unwrap();
        let (_, addr3) = inputs.iter().find(|(id, _)| id == "addr3").unwrap();
        //check that addr0 is not out of bounds and that it is the proper width!
        let addr0 = addr0.as_u64();
        let addr1 = addr1.as_u64();
        let addr2 = addr2.as_u64();
        let addr3 = addr3.as_u64();
        let real_addr = self.calc_addr(addr0, addr1, addr2, addr3);

        let old = self.data[real_addr as usize].clone();

        vec![
            (ir::Id::from("read_data"), old.into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
        ]
    }

    fn commit_updates(&mut self) {
        if let Some((addr0, addr1, addr2, addr3, val)) = self.update.take() {
            let real_addr = self.calc_addr(addr0, addr1, addr2, addr3);
            self.data[real_addr as usize] = val;
        }
    }

    fn clear_update_buffer(&mut self) {
        self.update = None;
    }
}

impl Serialize for StdMemD4 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mem = self
            .data
            .iter()
            .chunks((self.d3_size * self.d2_size * self.d1_size) as usize)
            .into_iter()
            .map(|x| {
                x.into_iter()
                    .chunks((self.d2_size * self.d1_size) as usize)
                    .into_iter()
                    .map(|y| {
                        y.into_iter()
                            .chunks(self.d1_size as usize)
                            .into_iter()
                            .map(|z| {
                                z.into_iter()
                                    .map(|val| val.as_u64())
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        mem.serialize(serializer)
    }
}

/// A Standard Register of a certain [width].

#[derive(Clone, Debug)]
pub struct StdReg {
    pub width: u64,
    pub val: Value,
    update: Option<Value>,
}

impl StdReg {
    /// New registers have unitialized values -- only specify their widths
    pub fn new(width: u64) -> StdReg {
        StdReg {
            width,
            val: Value::new(width as usize),
            update: None,
        }
    }

    /// warning unsafe deprecated
    pub fn read_value(&self) -> Value {
        self.val.clone()
    }

    /// warning unsafe deprecated
    pub fn read_u64(&self) -> u64 {
        self.val.as_u64()
    }
}

impl ValidateInput for StdReg {
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "in" => assert_eq!(v.len() as u64, self.width),
                "write_en" => assert_eq!(v.len(), 1),
                _ => {}
            }
        }
    }
}

impl ExecuteStateful for StdReg {
    fn execute_mut(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        current_done_val: &Value,
    ) -> Vec<(ir::Id, OutputValue)> {
        //unwrap the arguments
        let (_, input) = inputs.iter().find(|(id, _)| id == "in").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        //write the input to the register
        if write_en.as_u64() == 1 {
            self.update = Some((*input).clone());
            let old = self.val.clone();
            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new register data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("out"),
                    TimeLockedValue::new((*input).clone(), 1, Some(old)).into(),
                ),
                (
                    "done".into(),
                    PulseValue::new(
                        current_done_val.clone(),
                        Value::bit_high(),
                        Value::bit_low(),
                        1,
                    )
                    .into(),
                ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // OUT: the old value in the register, b/c we couldn't write
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![(ir::Id::from("out"), self.val.clone().into())]
        }
    }

    fn reset(
        &self,
        _inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        vec![
            (ir::Id::from("out"), self.val.clone().into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
        ]
    }

    fn commit_updates(&mut self) {
        if let Some(val) = self.update.take() {
            self.val = val;
        }
    }

    fn clear_update_buffer(&mut self) {
        self.update = None;
    }
}

impl Serialize for StdReg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let val = self.val.as_u64();
        val.serialize(serializer)
    }
}

/// A draft of a non-combinational, unsigned pipelined multiplication primitive
#[derive(Clone, Debug)]
pub struct StdMultPipe {
    pub width: u64,
    pub left: Value,
    pub right: Value,
    pub out: Value,
    update: Option<TimeLockedValue>, //make this Option TLV
                                     //so commit updates just dec count + writes
                                     //once count is 0
                                     //want to be able to re-execute StdMultPipe as many times as you want
                                     //before you tick over the clock. Then on (3rd) clock tick (?), the update
                                     //is written to [out]

                                     //should be able to re-assert left and right as much as you want;
                                     //the values will be captured on the clock tick. then those values need
                                     //to be left alone for 2 more cycles before the [out] is written to.
                                     //no guarantee for what happens if you change [left] and [right] during those
                                     //2 cyclesx
}

impl StdMultPipe {
    /// New StdMultPipe have zeroed left and right ports
    pub fn new(width: u64) -> StdMultPipe {
        StdMultPipe {
            width,
            left: Value::zeroes(width as usize),
            right: Value::zeroes(width as usize),
            out: Value::zeroes(width as usize),
            //note on out: if left and right are say 3-bit 7s, out will be cut off
            //so it's important the user gives left + right padded w/ enough zeroes
            //as in, it's important the user stay aware of overflow
            update: None,
        }
    }
}

impl ValidateInput for StdMultPipe {
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "left" => assert_eq!(v.len() as u64, self.width),
                "right" => assert_eq!(v.len() as u64, self.width),
                _ => {}
            }
        }
    }
}

//how std_mult_pipe works:
//you call execute_mut(left, right).
//can call that as many times as you want; StdMultPipe will
//only updates its [out] port once [commit_update] is issued
//but, this method still returns the output vector

impl ExecuteStateful for StdMultPipe {
    /// Writes an update to StdMultPipe based on the Value inputs [left] and [right].
    /// To commit this update, you must call [commit_updates]. Returns a vector
    /// of OutputValues (TLV of product, Pulse of done). The product will be of width
    /// twice as large as [left]/[right]/[self.width]
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// use calyx::ir;
    ///
    /// let mut mult_pipe = StdMultPipe::new(8); //inputs are 4-bit, output is 8
    /// let left = (ir::Id::from("left"), &Value::try_from_init(3, 8).unwrap());
    /// let right = (ir::Id::from("right"), &Value::try_from_init(8, 8).unwrap());
    /// let output_vals = mult_pipe.execute_mut(&[left, right], &Value::bit_low());
    /// let mut output_vals = output_vals.into_iter();
    /// let (out, done) = (output_vals.next().unwrap(), output_vals.next().unwrap());
    /// let mut out = out.1.unwrap_tlv();
    /// if let OutputValue::PulseValue(mut d) = done.1 {
    ///     assert_eq!(d.get_val().as_u64(), 0);
    ///     d.tick();
    ///     assert_eq!(d.get_val().as_u64(), 1);
    ///     d.tick();
    ///     assert_eq!(d.get_val().as_u64(), 0);
    /// } else {
    ///     panic!()
    /// }
    /// assert_eq!(out.get_count(), 3); //it takes 3 cycles to do pipelined mult
    /// out.dec_count();
    /// out.dec_count();
    /// out.dec_count();
    /// assert!(out.unlockable());
    /// assert_eq!(out.clone().unlock().as_u64(), Value::try_from_init(24, 8).unwrap().as_u64());
    /// ```
    /// # Panics
    /// Panics if [left], [right] are not the same width as self.width.
    fn execute_mut(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        current_done_val: &Value,
    ) -> Vec<(ir::Id, OutputValue)> {
        //unwrap the arguments -- no "write_en"
        let (_, left) = inputs.iter().find(|(id, _)| id == "left").unwrap();
        let (_, right) = inputs.iter().find(|(id, _)| id == "right").unwrap();
        //calculate the product -- no "write_en", so no if statement
        let product = left.as_u64() * right.as_u64();
        let update_val = Value::try_from_init(product, self.width).unwrap();
        let old = self.out.clone();
        self.update =
            Some(TimeLockedValue::new(update_val, 3, Some(old.clone())));
        // what's in this vector:
        // the "out" -- TimeLockedValue ofthe new mult data. Needs 3 cycles before readable (?)
        // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
        // all this coordination is done by the interpreter. We just set it up correctly
        vec![
            (
                ir::Id::from("out"),
                TimeLockedValue::new(
                    Value::try_from_init(product, self.width).unwrap(),
                    3,
                    Some(old),
                )
                .into(),
            ),
            (
                "done".into(), //rehaul this?
                //pulsevalues expect done to be asserted next cycle
                PulseValue::new(
                    current_done_val.clone(),
                    Value::bit_high(),
                    Value::bit_low(),
                    1,
                )
                .into(),
            ),
        ]
    }

    fn reset(
        &self,
        _inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        vec![
            (ir::Id::from("out"), self.out.clone().into()),
            (ir::Id::from("done"), Value::zeroes(1).into()),
        ]
    }

    /// Currently both a [commit_updates] and decremnter for the TLV in the update
    ///
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// use calyx::ir;
    ///
    /// let mut mult_pipe = StdMultPipe::new(8);
    /// let left = (ir::Id::from("left"), &Value::try_from_init(3, 8).unwrap());
    /// let right = (ir::Id::from("right"), &Value::try_from_init(8, 8).unwrap());
    /// mult_pipe.execute_mut(&[left, right], &Value::bit_low());
    /// mult_pipe.commit_updates();
    /// assert_eq!(mult_pipe.out.as_u64(), 0); //should not have been written to yet
    /// mult_pipe.commit_updates();
    /// mult_pipe.commit_updates();
    /// assert_eq!(mult_pipe.out.as_u64(), 24); // has been 3 cycles, should have been written to
    /// mult_pipe.commit_updates();
    /// assert_eq!(mult_pipe.out.as_u64(), 24); // committing no updates is fine
    /// ```
    fn commit_updates(&mut self) {
        //while TLV in update buffer isn't ready, assign old
        //once ready (this method does the ticks), assign
        //this update is set in [execute_mut]
        if let Some(mut out) = self.update.take() {
            //make TLV in update closer to being ready
            out.dec_count();
            if out.unlockable() {
                self.out = out.unlock();
                //leave self.update as None?
            } else {
                //assign old val
                //can only do this once; first time you do it,
                //the old value will be replaced
                if let Some(old_val) = out.old_value.take() {
                    self.out = old_val;
                }
                self.update = Some(out);
            }
        }
    }

    fn clear_update_buffer(&mut self) {
        self.update = None;
    }
}

impl Serialize for StdMultPipe {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        //don't really know the serializer... can we be more explicit, given
        //there are only 3 values?
        let mem = vec![&self.left, &self.right, &self.out]
            .iter()
            .chunks(self.width as usize)
            .into_iter()
            .map(|x| x.into_iter().map(|y| y.as_u64()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        mem.serialize(serializer)
    }
}

/// A component that keeps one value, that can't be rewritten. Is immutable,
/// and instantiated with the value it holds, which must have the same # of bits as [width].
#[derive(Clone, Debug)]
pub struct StdConst {
    width: u64,
    val: Value,
}

impl StdConst {
    /// Instantiates a new constant component
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let const_16bit_9 = StdConst::new(16, Value::try_from_init(9, 16).unwrap());
    /// ```
    ///
    /// # Panics
    /// * Panics if [val]'s width != [width]
    pub fn new(width: u64, val: Value) -> StdConst {
        check_widths(&val, &val, width);
        StdConst { width, val }
    }

    /// Returns the value this constant component represents
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let const_16bit_9 = StdConst::new(16, Value::try_from_init(9, 16).unwrap());
    /// let val_9 = const_16bit_9.read_val();
    /// ```
    pub fn read_val(&self) -> Value {
        self.val.clone()
    }

    /// Returns the u64 corresponding to the value this constant component represents
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let const_16bit_9 = StdConst::new(16, Value::try_from_init(9, 16).unwrap());
    /// assert_eq!(const_16bit_9.read_u64(), 9);
    /// ```
    pub fn read_u64(&self) -> u64 {
        self.val.as_u64()
    }
}

///std_lsh<WIDTH>
///A left bit shift accepting only inputs of [width]. Performs LEFT << RIGHT. This component is combinational.
///Inputs:
///left: WIDTH - A WIDTH-bit value to be shifted
///right: WIDTH - A WIDTH-bit value representing the shift amount
///Outputs:
///out: WIDTH - A WIDTH-bit value equivalent to LEFT << RIGHT
#[derive(Clone, Debug)]
pub struct StdLsh {
    width: u64,
}

impl StdLsh {
    /// Instantiate a new StdLsh of a specific width
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_lsh_16_bit = StdLsh::new(16);
    /// ```
    pub fn new(width: u64) -> StdLsh {
        StdLsh { width }
    }
}

impl ExecuteBinary for StdLsh {
    /// Returns the Value representing LEFT << RIGHT
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let std_lsh_16_bit = StdLsh::new(16);
    /// let val_2_16bit = Value::try_from_init(2, 16).unwrap();
    /// let val_8_16bit = std_lsh_16_bit.execute_bin(&val_2_16bit, &val_2_16bit);
    /// assert_eq!(val_8_16bit.unwrap_imm().as_u64(), 8);
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        //to avoid the casting overflow,
        //we know that [left], [right], and [self]
        //are capped at bitwidths as large as largest u64 (2^64 - 1 = 1.84 * 10^19 ...)
        //so, if [self] has a width greater than 64,
        //and the 65th index is a 1, we just automatically return a 0 of the
        //appropriate bitwidth!

        if self.width > 64 {
            //check if right is greater than or equal to  2 ^ 64
            let r_vec = &right.vec;
            let mut index: u64 = 0;
            for bit in r_vec.iter().by_ref() {
                if (index >= 64) & *bit {
                    return Value::zeroes(self.width as usize).into();
                }
                index += 1;
            }
        }

        //but that's not the only problem. we can't let [place] get to
        //2^64 or above. However the right value couldn't have even been specified
        //to be greater than or equal to 2^64, because it's constrained by u64.
        //so instead of incrementing [place], just calculate place, but only
        //when [bit] is 1, which can only be true for bits below the 65th (first
        // bit is 2^0)

        let mut index: u32 = 0;
        let mut tr = BitVec::new();
        //first push the requisite # of zeroes
        for bit in right.vec.iter().by_ref() {
            if *bit {
                //not possible for bit to be 1 after the 64th bit
                for _ in 0..u64::pow(2, index) {
                    if tr.len() < self.width as usize {
                        tr.push(false);
                    }
                    //no point in appending once we've filled it all with zeroes
                }
            }
            index += 1;
        }
        //then copy over the bits from [left] onto the back (higher-place bits) of
        //[tr]. Then truncate, aka slicing off the bits that exceed the width of this
        //component
        let mut to_append = left.clone().vec;
        tr.append(&mut to_append);
        tr.truncate(self.width as usize);
        let tr = Value { vec: tr };
        assert_eq!(tr.width(), self.width);
        //sanity check the widths
        tr.into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/// std_rsh<WIDTH>
/// A right bit shift. Performs LEFT >> RIGHT. This component is combinational.

/// Inputs:

/// left: WIDTH - A WIDTH-bit value to be shifted
/// right: WIDTH - A WIDTH-bit value representing the shift amount
/// Outputs:

/// out: WIDTH - A WIDTH-bit value equivalent to LEFT >> RIGHT
#[derive(Clone, Debug)]
pub struct StdRsh {
    width: u64,
}

impl StdRsh {
    /// Instantiate a new StdRsh component with a specified width
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_rsh_16bit = StdRsh::new(16);
    /// ```
    pub fn new(width: u64) -> StdRsh {
        StdRsh { width }
    }
}

impl ExecuteBinary for StdRsh {
    /// Returns the Value representing LEFT >> RIGHT
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let std_rsh_16_bit = StdRsh::new(16);
    /// let val_8_16bit = Value::try_from_init(8, 16).unwrap();
    /// let val_1_16bit = Value::try_from_init(1, 16).unwrap();
    /// let val_4_16bit = std_rsh_16_bit.execute_bin(&val_8_16bit, &val_1_16bit);
    /// assert_eq!(val_4_16bit.unwrap_imm().as_u64(), 4);
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        //remove [right] bits from index 0
        //extend to proper size

        //same check as in LSH
        if self.width > 64 {
            //check if right is greater than or equal to  2 ^ 64
            let r_vec = &right.vec;
            let mut index: u64 = 0;
            for bit in r_vec.iter().by_ref() {
                if (index >= 64) & *bit {
                    return Value::zeroes(self.width as usize).into();
                }
                index += 1;
            }
        }

        let mut index: u32 = 0;
        let mut tr = left.vec.clone();
        //first remove [right] bits
        for bit in right.vec.iter().by_ref() {
            if *bit {
                for _ in 0..u64::pow(2, index) {
                    if tr.len() > 0 {
                        tr.remove(0);
                    }
                }
            }
            index += 1;
        }
        //now resize to proper size, putting 0s at the end (0 is false)
        tr.resize(self.width as usize, false);
        let tr = Value { vec: tr };
        assert_eq!(tr.width(), self.width);
        //sanity check the widths
        tr.into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

///signed bitnum greater than comparison
#[derive(Clone, Debug)]
pub struct StdSgt {
    width: u64,
}

impl StdSgt {
    /// Instantiate a new StdSgt with a specified bit width
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_sgt_16bit = StdSgt::new(16);
    /// ```
    pub fn new(width: u64) -> StdSgt {
        StdSgt { width }
    }
}

impl ExecuteBinary for StdSgt {
    /// Returns a one-bit 1 if LEFT > RIGHT, else 0. Interprets LEFT and RIGHT
    /// as signed bitnums, meaning the bits of LEFT and RIGHT are read as two's
    /// complement representation of integers.
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let std_sgt_4bit = StdSgt::new(4);
    /// let val_neg_1_4bit = Value::try_from_init(15, 4).unwrap(); //[1111]
    /// let val_4_4bit = Value::try_from_init(4, 4).unwrap(); //[0100]
    /// let val_0_4bit = std_sgt_4bit.execute_bin(&val_neg_1_4bit, &val_4_4bit).unwrap_imm();
    /// assert_eq!(val_0_4bit.as_u64(), 0);
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        let left_64 = left.as_i64();
        let right_64 = right.as_i64();
        if left_64 > right_64 {
            Value::bit_high().into()
        } else {
            Value::bit_low().into()
        }
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

//std_add<WIDTH>
//Bitwise addition without a carry flag. Performs LEFT + RIGHT. This component is combinational.
//Inputs:
//left: WIDTH - A WIDTH-bit value
//right: WIDTH - A WIDTH-bit value
//Outputs:
//out: WIDTH - A WIDTH-bit value equivalent to LEFT + RIGHT
#[derive(Clone, Debug)]
pub struct StdAdd {
    width: u64,
}

impl StdAdd {
    /// Instantiate a new StdAdd with a specified bit width
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_add_16bit = StdAdd::new(16);
    /// ```
    pub fn new(width: u64) -> StdAdd {
        StdAdd { width }
    }
}

impl ExecuteBinary for StdAdd {
    /// Returns the Value representing LEFT + RIGHT
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let std_add_16bit = StdAdd::new(16);
    /// let val_8_16bit = Value::try_from_init(8, 16).unwrap();
    /// let val_1_16bit = Value::try_from_init(1, 16).unwrap();
    /// let val_9_16bit = std_add_16bit.execute_bin(&val_8_16bit, &val_1_16bit);
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        let a_iter = left.vec.iter().by_ref();
        let b_iter = right.vec.iter().by_ref();
        let mut c_in = false;
        let mut sum = BitVec::new();
        for (ai, bi) in a_iter.zip(b_iter) {
            sum.push(
                c_in & !ai & !bi
                    || bi & !c_in & !ai
                    || ai & !c_in & !bi
                    || ai & bi & c_in,
            );
            c_in = bi & c_in || ai & c_in || ai & bi || ai & c_in & bi;
        }
        let tr = Value { vec: sum };
        //as a sanity check, check tr has same width as left
        assert_eq!(tr.width(), left.width());
        tr.into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/// std_sub<WIDTH>
/// Bitwise subtraction. Performs LEFT - RIGHT. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit value
/// right: WIDTH - A WIDTH-bit value
/// Outputs:
/// out: WIDTH - A WIDTH-bit value equivalent to LEFT - RIGHT
#[derive(Clone, Debug)]
pub struct StdSub {
    width: u64,
}

impl StdSub {
    /// Instantiates a new standard subtraction component
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_sub_16bit = StdSub::new(16);
    /// ```
    pub fn new(width: u64) -> StdSub {
        StdSub { width }
    }
}

impl ExecuteBinary for StdSub {
    /// Returns the Value representing LEFT - RIGHT
    /// Will overflow if result is negative.
    /// # Examples
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// //4 [0100] - 1 [0001] = 3 [0011]
    /// let val_4_4bit = Value::try_from_init(4, 4).unwrap();
    /// let val_1_4bit = Value::try_from_init(1, 4).unwrap();
    /// let std_sub_4_bit = StdSub::new(4);
    /// let val_3_4bit = std_sub_4_bit.execute_bin(&val_4_4bit, &val_1_4bit).unwrap_imm();
    /// assert_eq!(val_3_4bit.as_u64(), 3);
    /// //4 [0100] - 5 [0101] = -1 [1111] <- as an unsigned binary num, this is 15
    /// let val_5_4bit = Value::try_from_init(5, 4).unwrap();
    /// let res = std_sub_4_bit.execute_bin(&val_4_4bit, &val_5_4bit).unwrap_imm();
    /// assert_eq!(res.as_u64(), 15);
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        //first turn right into ~right + 1
        let new_right = !right.vec.clone();
        let adder = StdAdd::new(self.width + 1);
        let new_right = adder
            .execute_bin(
                &Value { vec: new_right },
                &Value::try_from_init(1, self.width).unwrap(),
            )
            .unwrap_imm();

        //now do addition. maybe better to use the adder and unwrap the OutputValue?
        let a_iter = left.vec.iter().by_ref();
        let b_iter = new_right.vec.iter().by_ref();
        let mut c_in = false;
        let mut sum = BitVec::new();
        for (ai, bi) in a_iter.zip(b_iter) {
            sum.push(
                c_in & !ai & !bi
                    || bi & !c_in & !ai
                    || ai & !c_in & !bi
                    || ai & bi & c_in,
            );
            c_in = bi & c_in || ai & c_in || ai & bi || ai & c_in & bi;
        }
        //actually ok if there is overflow from 2sc subtraction (?)
        //have to check if this is ok behavior
        return Value { vec: sum }.into();
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/// Slice out the lower OUT_WIDTH bits of an IN_WIDTH-bit value. Computes
/// in[out_width - 1 : 0]. This component is combinational.
/// Inputs:
/// in: IN_WIDTH - An IN_WIDTH-bit value
/// Outputs:
/// out: OUT_WIDTH - The lower (from LSB towards MSB) OUT_WIDTH bits of in
#[derive(Clone, Debug)]
pub struct StdSlice {
    in_width: u64,
    out_width: u64,
}

impl StdSlice {
    /// Instantiate a new instance of StdSlice
    ///
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_slice_6_to_4 = StdSlice::new(6, 4);
    /// ```
    pub fn new(in_width: u64, out_width: u64) -> StdSlice {
        StdSlice {
            in_width,
            out_width,
        }
    }
}

impl ValidateInput for StdSlice {
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        let in_width =
            inputs.iter().find_map(
                |(id, v)| {
                    if id == "in" {
                        Some(v)
                    } else {
                        None
                    }
                },
            );

        if let Some(v) = in_width {
            assert_eq!(v.len() as u64, self.in_width)
        }
    }
}

impl ExecuteUnary for StdSlice {
    /// Returns the bottom OUT_WIDTH bits of an input with IN_WIDTH
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_5_3bits = Value::try_from_init(5, 3).unwrap(); // 5 = [101]
    /// let std_slice_3_to_2 = StdSlice::new(3, 2);
    /// let val_1_2bits = std_slice_3_to_2.execute_unary(&val_5_3bits); // 1 = [01]
    /// ```
    ///
    /// # Panics
    /// * panics if input's width and self.width are not equal
    ///
    fn execute_unary(&self, input: &Value) -> OutputValue {
        let tr = input.clone();
        tr.truncate(self.out_width as usize).into()
    }
}

/// Given an IN_WIDTH-bit input, zero pad from the MSB to an output of
/// OUT_WIDTH-bits. This component is combinational.
/// Inputs:
/// in: IN_WIDTH - An IN_WIDTH-bit value to be padded
/// Outputs:
/// out: OUT_WIDTH - The paddwd width
#[derive(Clone, Debug)]
pub struct StdPad {
    in_width: u64,
    out_width: u64,
}

impl StdPad {
    /// Instantiate instance of StdPad that takes input with width [in_width] and returns output with width [out_width]
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_pad_3_to_5 = StdPad::new(3, 5);
    /// ```
    pub fn new(in_width: u64, out_width: u64) -> StdPad {
        StdPad {
            in_width,
            out_width,
        }
    }
}

impl ValidateInput for StdPad {
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        let in_width =
            inputs.iter().find_map(
                |(id, v)| {
                    if id == "in" {
                        Some(v)
                    } else {
                        None
                    }
                },
            );

        if let Some(v) = in_width {
            assert_eq!(v.len() as u64, self.in_width)
        }
    }
}

impl ExecuteUnary for StdPad {
    /// Returns a value of length OUT_WIDITH consisting IN_WIDTH bits corresponding
    /// with [input], padded with 0s until index OUT_WIDTH - 1
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_5_3bits = Value::try_from_init(5, 3).unwrap(); // 5 = [101]
    /// let std_pad_3_to_5 = StdPad::new(3, 5);
    /// let val_5_5bits = std_pad_3_to_5.execute_unary(&val_5_3bits); // 5 = [00101]
    /// ```
    ///
    /// # Panics
    /// * panics if input's width and self.width are not equal
    ///
    fn execute_unary(&self, input: &Value) -> OutputValue {
        let pd = input.clone();
        pd.ext(self.out_width as usize).into()
    }
}

/* =========================== Logical Operators =========================== */
/// std_not<WIDTH>
/// Bitwise NOT. This component is combinational.
/// Inputs:
/// in: WIDTH - A WIDTH-bit input.
/// Outputs:
/// out: WIDTH - The bitwise NOT of the input (~in)
#[derive(Clone, Debug)]
pub struct StdNot {
    width: u64,
}

impl StdNot {
    /// Instantiate a standard not component accepting input of width [WIDTH]
    pub fn new(width: u64) -> StdNot {
        StdNot { width }
    }
}

impl ValidateInput for StdNot {
    fn validate_input(&self, inputs: &[(ir::Id, &Value)]) {
        let in_width =
            inputs.iter().find_map(
                |(id, v)| {
                    if id == "in" {
                        Some(v)
                    } else {
                        None
                    }
                },
            );

        if let Some(v) = in_width {
            assert_eq!(v.len() as u64, self.width)
        }
    }
}

impl ExecuteUnary for StdNot {
    /// Returns a value of length WIDTH representing the bitwise NOT of [input]
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_5_3bits = Value::try_from_init(5, 3).unwrap(); // 5 = [101]
    /// let std_not_3bit = StdNot::new(3);
    /// let val_2_3bits = std_not_3bit.execute_unary(&val_5_3bits).unwrap_imm();
    /// assert_eq!(val_2_3bits.as_u64(), 2);
    /// ```
    ///
    /// # Panics
    /// * panics if input's width and self.width are not equal
    ///
    fn execute_unary<'a>(&self, input: &Value) -> OutputValue {
        Value {
            vec: input.vec.clone().not(),
        }
        .into()
    }
}

/// std_and<WIDTH>
/// Bitwise AND. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:

// out: WIDTH - The bitwise AND of the arguments (left & right)
#[derive(Clone, Debug)]
pub struct StdAnd {
    width: u64,
}

impl StdAnd {
    /// Instantiate an instance of StdAnd that accepts input of width [width]
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_and_4bit = StdAdd::new(4);
    /// ```
    pub fn new(width: u64) -> StdAnd {
        StdAnd { width }
    }
}

impl ExecuteBinary for StdAnd {
    /// Returns the bitwise AND of [left] and [right]
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_5_3 = Value::try_from_init(5, 3).unwrap();
    /// let val_2_3 = Value::try_from_init(2, 3).unwrap();
    /// let std_and_3bit = StdAnd::new(3);
    /// let val_0_3 = std_and_3bit.execute_bin(&val_5_3, &val_2_3).unwrap_imm();
    /// assert_eq!(val_0_3.as_u64(), 0);
    /// ```
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        Value {
            vec: left.vec.clone() & right.vec.clone(),
        }
        .into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/// std_or<WIDTH>
/// Bitwise OR. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: WIDTH - The bitwise OR of the arguments (left | right)
#[derive(Clone, Debug)]
pub struct StdOr {
    width: u64,
}

impl StdOr {
    /// Instantiate a StdOr that accepts inputs only with width [width]
    pub fn new(width: u64) -> StdOr {
        StdOr { width }
    }
}

impl ExecuteBinary for StdOr {
    /// Returns the bitwise OR of [left] and [right]
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_5_3 = Value::try_from_init(5, 3).unwrap(); // 5 = [101]
    /// let val_2_3 = Value::try_from_init(2, 3).unwrap(); // 2 = [010]
    /// let std_or_3bit = StdOr::new(3);
    /// let val_7_3 = std_or_3bit.execute_bin(&val_5_3, &val_2_3).unwrap_imm();
    /// assert_eq!(val_7_3.as_u64(), 7);
    /// ```
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        Value {
            vec: left.vec.clone() | right.vec.clone(),
        }
        .into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/// std_xor<WIDTH>
/// Bitwise XOR. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: WIDTH - The bitwise XOR of the arguments (left ^ right)
#[derive(Clone, Debug)]
pub struct StdXor {
    width: u64,
}

impl StdXor {
    /// Instantiate a StdXor component that accepts only inputs of width [width]
    pub fn new(width: u64) -> StdXor {
        StdXor { width }
    }
}

impl ExecuteBinary for StdXor {
    /// Returns the bitwise XOR of [left] and [right]
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_7_3 = Value::try_from_init(7, 3).unwrap(); // 7 = [111]
    /// let val_2_3 = Value::try_from_init(2, 3).unwrap(); // 2 = [010]
    /// let std_xor_3bit = StdXor::new(3);
    /// let val_5_3 = std_xor_3bit.execute_bin(&val_7_3, &val_2_3).unwrap_imm();
    /// assert_eq!(val_5_3.as_u64(), 5);
    /// ```
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        Value {
            vec: left.vec.clone() ^ right.vec.clone(),
        }
        .into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/* ========================== Comparison Operators ========================= */
/// std_gt<WIDTH>
/// Greater than. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: 1 - A single bit output. 1 if left > right else 0.
#[derive(Clone, Debug)]
pub struct StdGt {
    width: u64,
}

impl StdGt {
    /// Instantiate a StdGt component that accepts only inputs with width [width]
    pub fn new(width: u64) -> StdGt {
        StdGt { width }
    }
}

impl ExecuteBinary for StdGt {
    /// Returns a single bit-long Value which is 1 if left > right else 0
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_gt_3bit = StdGt::new(3);
    /// let res = std_gt_3bit.execute_bin(&val_2_3bit, &val_1_3bit).unwrap_imm();
    /// assert_eq!(res.as_u64(), 1);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 > right_64;

        Value::from_init(init_val, 1_usize).into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/// std_lt<WIDTH>
/// Less than. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: 1 - A single bit output. 1 if left < right else 0.
#[derive(Clone, Debug)]
pub struct StdLt {
    width: u64,
}

impl StdLt {
    /// Instantiate a StdLt component that only accepts inputs of width [width]
    pub fn new(width: u64) -> StdLt {
        StdLt { width }
    }
}

impl ExecuteBinary for StdLt {
    /// Returns a single bit-long Value which is 1 if left < right else 0
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_lt_3bit = StdLt::new(3);
    /// let res = std_lt_3bit.execute_bin(&val_2_3bit, &val_1_3bit).unwrap_imm();
    /// assert_eq!(res.as_u64(), 0);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 < right_64;

        Value::from_init(init_val, 1_usize).into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/// std_eq<WIDTH>
/// Equality comparison. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: 1 - A single bit output. 1 if left = right else 0.
#[derive(Clone, Debug)]
pub struct StdEq {
    width: u64,
}

impl StdEq {
    /// Instantiates a StdEq that only accepts inputs of width [width]
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_eq_4bit = StdEq::new(4);
    /// ```
    pub fn new(width: u64) -> StdEq {
        StdEq { width }
    }
}

impl ExecuteBinary for StdEq {
    /// Returns a single bit-long Value which is 1 if left == right else 0
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_eq_3bit = StdEq::new(3);
    /// let res = std_eq_3bit.execute_bin(&val_2_3bit, &val_1_3bit).unwrap_imm();
    /// assert_eq!(res.as_u64(), 0);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 == right_64;

        Value::from_init(init_val, 1_usize).into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/// std_neq<WIDTH>
/// Not equal. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: 1 - A single bit output. 1 if left != right else 0.
///
#[derive(Clone, Debug)]
pub struct StdNeq {
    width: u64,
}

impl StdNeq {
    /// Instantiates a StdNeq component that only accepts inputs of width [width]
    /// /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_neq_4bit = StdNeq::new(4);
    /// ```
    pub fn new(width: u64) -> StdNeq {
        StdNeq { width }
    }
}

impl ExecuteBinary for StdNeq {
    /// Returns a single bit-long Value which is 1 if left != right else 0
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_neq_3bit = StdNeq::new(3);
    /// let res = std_neq_3bit.execute_bin(&val_2_3bit, &val_1_3bit).unwrap_imm();
    /// assert_eq!(res.as_u64(), 1);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 != right_64;
        Value::from_init(init_val, 1_usize).into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/// std_ge<WIDTH>
/// Greater than or equal. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: 1 - A single bit output. 1 if left >= right else 0.
#[derive(Clone, Debug)]
pub struct StdGe {
    width: u64,
}
impl StdGe {
    /// Instantiate a new StdGe component that accepts only inputs of width [width]
    /// /// # Example
    /// ```
    /// use interp::primitives::*;
    /// let std_Ge_4bit = StdGe::new(4);
    /// ```
    pub fn new(width: u64) -> StdGe {
        StdGe { width }
    }
}
impl ExecuteBinary for StdGe {
    /// Returns a single bit-long Value which is 1 if left >= right else 0
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_ge_3bit = StdGe::new(3);
    /// let res = std_ge_3bit.execute_bin(&val_2_3bit, &val_1_3bit).unwrap_imm();
    /// assert_eq!(res.as_u64(), 1);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 >= right_64;

        Value::from_init(init_val, 1_usize).into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}

/// std_le<WIDTH>
/// Less than or equal. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: 1 - A single bit output. 1 if left <= right else 0.
#[derive(Clone, Debug)]
pub struct StdLe {
    width: u64,
}

impl StdLe {
    /// Instantiate a StdLe component that only accepts inputs of width [width]
    pub fn new(width: u64) -> StdLe {
        StdLe { width }
    }
}

impl ExecuteBinary for StdLe {
    /// Returns a single bit-long Value which is 1 if left <= right else 0
    /// # Example
    /// ```
    /// use interp::primitives::*;
    /// use interp::values::*;
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_le_3bit = StdLe::new(3);
    /// let res = std_le_3bit.execute_bin(&val_2_3bit, &val_1_3bit).unwrap_imm();
    /// assert_eq!(res.as_u64(), 0);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 <= right_64;
        Value::from_init(init_val, 1_usize).into()
    }

    fn get_width(&self) -> &u64 {
        &self.width
    }
}
