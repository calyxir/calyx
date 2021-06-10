//! Defines update methods for the various primitive cells in the Calyx
// standard library.
use super::values::{OutputValue, TimeLockedValue, Value};
use calyx::ir;
use std::convert::TryInto;
use std::ops::*;

#[derive(Clone, Debug)]
pub enum Primitive {
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
pub trait ExecuteUnary {
    fn execute_unary(&self, input: &Value) -> OutputValue;

    /// Default implementation of [execute] for all unary components
    /// Unwraps inputs, then sends output based on [execute_unary]
    fn execute(
        &self,
        inputs: &[(ir::Id, Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        let (_, input) = inputs.iter().find(|(id, _)| id == "in").unwrap();
        vec![(ir::Id::from("out"), self.execute_unary(input))]
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

    /// Default implementation of [execute] for all binary components
    /// Unwraps inputs (left and right), then sends output based on [execute_bin]
    fn execute<'a>(
        &self,
        inputs: &'a [(ir::Id, Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        let (_, left) = inputs.iter().find(|(id, _)| id == "left").unwrap();

        let (_, right) = inputs.iter().find(|(id, _)| id == "right").unwrap();

        let out = self.execute_bin(left, right);
        vec![(ir::Id::from("out"), out)]
    }
}

/// Only binary operator components have trait [Execute].
pub trait Execute {
    fn execute<'a>(
        &self,
        inputs: &'a [(ir::Id, Value)],
    ) -> Vec<(ir::Id, OutputValue)>;
}

/// ExecuteStateful is a trait implemnted by primitive components such as
/// StdReg and StdMem (D1 -- D4), allowing their state to be modified.
pub trait ExecuteStateful {
    /// Use execute_mut to modify the state of a stateful component.
    /// No restrictions on exactly how the input(s) look
    fn execute_mut<'a>(
        &mut self,
        inputs: &'a [(ir::Id, Value)], //TODO: maybe change these to immutable references?
    ) -> Vec<(ir::Id, OutputValue)>;
}

/// Ensures the input values are of the appropriate widths, else panics.
fn check_widths(left: &Value, right: &Value, width: u64) -> () {
    if width != (left.vec.len() as u64)
        || width != (right.vec.len() as u64)
        || left.vec.len() != right.vec.len()
    {
        panic!("Width mismatch between the component and the inputs.");
    }
}

/// Ensures that the input value is of the appropriate width, else panics.
fn check_width(input: &Value, width: u64) {
    if width != (input.vec.len() as u64) {
        panic!("Width mismatch between the component and the input")
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
}

impl StdMemD1 {
    /// Instantiates a new StdMemD1 storing data of width [width], containing [size]
    /// slots for memory, accepting indecies (addr0) of width [idx_size].
    /// Note: if [idx_size] is smaller than the length of [size]'s binary representation,
    /// you will not be able to access the slots near the end of the memory.
    pub fn new(width: u64, size: u64, idx_size: u64) -> StdMemD1 {
        let data = vec![
            Value::zeroes((width as usize).try_into().unwrap());
            (size as usize).try_into().unwrap()
        ];
        StdMemD1 {
            width,
            size,     //how many slots of memory in the vector
            idx_size, //the width of the values used to address the memory
            data,
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
    /// let write_data = (ir::Id::from("write_data"), Value::try_from_init(1, 1).unwrap());
    /// let write_en = (ir::Id::from("write_en"), Value::try_from_init(1, 1).unwrap());
    /// let addr0 = (ir::Id::from("addr0"), Value::try_from_init(0, 3).unwrap());
    /// let output_vals = std_memd1.execute_mut(&[write_data, write_en, addr0]);
    /// let mut output_vals = output_vals.into_iter();
    /// let (read_data, done) = (output_vals.next().unwrap(), output_vals.next().unwrap());
    /// let mut rd = read_data.1.unwrap_tlv();
    /// let mut d = done.1.unwrap_tlv();
    /// assert_eq!(rd.get_count(), 1);
    /// assert_eq!(d.get_count(), 1);
    /// rd.dec_count(); d.dec_count();
    /// assert!(rd.unlockable());
    /// assert_eq!(rd.clone().unlock().as_u64(), Value::try_from_init(1, 1).unwrap().as_u64());
    /// ```
    /// # Panics
    /// Panics if [write_data] is not the same width as self.width. Panics if the width of addr0 does not equal
    /// self.idx_size.
    fn execute_mut(
        &mut self,
        inputs: &[(ir::Id, Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        //unwrap the arguments
        //these come from the primitive definition in verilog
        //don't need to depend on the user -- just make sure this matches primitive + calyx + verilog defs
        let (_, input) =
            inputs.iter().find(|(id, _)| id == "write_data").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        let (_, addr0) = inputs.iter().find(|(id, _)| id == "addr0").unwrap();
        //check that addr0 is not out of bounds and that it is the proper width!
        check_widths(addr0, addr0, self.idx_size); //make a unary one instead of hacking. Also change the panicking
                                                   //so we don't have to keep using .as_u64()
        let addr0 = addr0.as_u64();
        if addr0 >= self.size {
            panic!(
                "memory only has {} slots, addr0 tries to access slot {}",
                self.size, addr0
            );
        }
        //check that input data is the appropriate width as well
        check_width(input, self.width);
        let old = self.data[addr0 as usize].clone();
        // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.data[addr0 as usize] = input.clone();
            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new(
                        self.data[addr0 as usize].clone(),
                        1,
                        Some(old),
                    )
                    .into(),
                ),
                (
                    ir::Id::from("done"),
                    TimeLockedValue::new(
                        Value::try_from_init(1, 1).unwrap(),
                        1,
                        Some(Value::zeroes(1)),
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
                (ir::Id::from("done"), Value::zeroes(1).into()),
            ]
        }
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
    pub data: Vec<Vec<Value>>,
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
        let data = vec![
            vec![
                Value::zeroes((width as usize).try_into().unwrap());
                (d1_size as usize).try_into().unwrap()
            ];
            (d0_size as usize).try_into().unwrap()
        ];
        StdMemD2 {
            width,
            d0_size,
            d1_size,
            d0_idx_size,
            d1_idx_size,
            data,
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
        inputs: &[(ir::Id, Value)],
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
        check_width(input, self.width);
        //check that addr0 is not out of bounds and that it is the proper width!
        check_widths(addr0, addr0, self.d0_idx_size); //make a unary one instead of hacking. Also change the panicking
        let addr0 = addr0.as_u64();
        if addr0 >= self.d0_size {
            panic!(
                "memory only has {} slots, addr0 tries to access slot {}",
                self.d0_size, addr0
            );
        }
        //chech that addr1 is not out of bounds and that it is the proper iwdth
        check_widths(addr1, addr1, self.d1_idx_size); //make a unary one instead of hacking. Also change the panicking
        let addr1 = addr1.as_u64();
        if addr1 >= self.d1_size {
            panic!(
                "memory only has {} slots, addr1 tries to access slot {}",
                self.d1_size, addr1
            );
        }
        let old = self.data[addr0 as usize][addr1 as usize].clone(); //not sure if this could lead to errors (Some(old)) is borrow?
                                                                     // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.data[addr0 as usize][addr1 as usize] = input.clone();
            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new(
                        self.data[addr0 as usize][addr1 as usize].clone(),
                        1,
                        Some(old),
                    )
                    .into(),
                ),
                (
                    ir::Id::from("done"),
                    TimeLockedValue::new(
                        Value::try_from_init(1, 1).unwrap(),
                        1,
                        Some(Value::zeroes(1)),
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
                (ir::Id::from("done"), Value::zeroes(1).into()),
            ]
        }
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
    data: Vec<Vec<Vec<Value>>>,
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
        let data =
            vec![
                vec![
                    vec![
                        Value::zeroes((width as usize).try_into().unwrap());
                        (d2_size as usize).try_into().unwrap()
                    ];
                    (d1_size as usize).try_into().unwrap()
                ];
                (d0_size as usize).try_into().unwrap()
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
        inputs: &[(ir::Id, Value)],
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
        //chec that write_data is the exact right width
        check_width(input, self.width);
        //check that addr0 is not out of bounds and that it is the proper width!
        check_widths(addr0, addr0, self.d0_idx_size); //make a unary one instead of hacking. Also change the panicking
        let addr0 = addr0.as_u64();
        if addr0 >= self.d0_size {
            panic!(
                "memory only has {} slots, addr0 tries to access slot {}",
                self.d0_size, addr0
            );
        }
        //chech that addr1 is not out of bounds and that it is the proper iwdth
        check_widths(addr1, addr1, self.d1_idx_size); //make a unary one instead of hacking. Also change the panicking
        let addr1 = addr1.as_u64();
        if addr1 >= self.d1_size {
            panic!(
                "memory only has {} slots, addr1 tries to access slot {}",
                self.d1_size, addr1
            );
        }
        //check that addr2 is not out of bounds and that it is the proper width
        check_width(addr2, self.d2_idx_size);
        let addr2 = addr2.as_u64();
        if addr2 >= self.d2_size {
            panic!(
                "memory only has {} slots, addr2 tries to access slot {}",
                self.d2_size, addr2
            );
        }
        let old =
            self.data[addr0 as usize][addr1 as usize][addr2 as usize].clone(); //not sure if this could lead to errors (Some(old)) is borrow?
                                                                               // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.data[addr0 as usize][addr1 as usize][addr2 as usize] =
                input.clone();
            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new(input.clone(), 1, Some(old)).into(),
                ),
                (
                    ir::Id::from("done"),
                    TimeLockedValue::new(
                        Value::try_from_init(1, 1).unwrap(),
                        1,
                        Some(Value::zeroes(1)),
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
                (ir::Id::from("done"), Value::zeroes(1).into()),
            ]
        }
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
    data: Vec<Vec<Vec<Vec<Value>>>>,
}

impl StdMemD4 {
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
        let data =
            vec![
                vec![
                    vec![
                        vec![
                            Value::zeroes((width as usize).try_into().unwrap());
                            (d3_size as usize).try_into().unwrap()
                        ];
                        (d2_size as usize).try_into().unwrap()
                    ];
                    (d1_size as usize).try_into().unwrap()
                ];
                (d0_size as usize).try_into().unwrap()
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
        inputs: &[(ir::Id, Value)],
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
        //check that addr0 is not out of bounds and that it is the proper width!
        check_widths(addr0, addr0, self.d0_idx_size); //make a unary one instead of hacking. Also change the panicking
        let addr0 = addr0.as_u64();
        if addr0 >= self.d0_size {
            panic!(
                "memory only has {} slots, addr0 tries to access slot {}",
                self.d0_size, addr0
            );
        }
        //chec that write_data is the exact right width
        check_width(input, self.width);
        //chech that addr1 is not out of bounds and that it is the proper iwdth
        check_widths(addr1, addr1, self.d1_idx_size); //make a unary one instead of hacking. Also change the panicking
        let addr1 = addr1.as_u64();
        if addr1 >= self.d1_size {
            panic!(
                "memory only has {} slots, addr1 tries to access slot {}",
                self.d1_size, addr1
            );
        }
        //check that addr2 is not out of bounds and that it is the proper width
        check_width(addr2, self.d2_idx_size);
        let addr2 = addr2.as_u64();
        if addr2 >= self.d2_size {
            panic!(
                "memory only has {} slots, addr2 tries to access slot {}",
                self.d2_size, addr2
            );
        }
        //check that addr3 is not out of bounds and that it is the proper width
        check_width(addr3, self.d3_idx_size);
        let addr3 = addr3.as_u64();
        if addr3 >= self.d3_size {
            panic!(
                "memory only has {} slots, addr3 tries to access slot {}",
                self.d3_size, addr3
            )
        }
        let old = self.data[addr0 as usize][addr1 as usize][addr2 as usize]
            [addr3 as usize]
            .clone(); //not sure if this could lead to errors (Some(old)) is borrow?
                      // only write to memory if write_en is 1
        if write_en.as_u64() == 1 {
            self.data[addr0 as usize][addr1 as usize][addr2 as usize]
                [addr3 as usize] = input.clone();
            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new mem data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("read_data"),
                    TimeLockedValue::new(input.clone(), 1, Some(old)).into(),
                ),
                (
                    ir::Id::from("done"),
                    TimeLockedValue::new(
                        Value::try_from_init(1, 1).unwrap(),
                        1,
                        Some(Value::zeroes(1)),
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
                (ir::Id::from("done"), Value::zeroes(1).into()),
            ]
        }
    }
}

/// A Standard Register of a certain [width].
/// Rules regarding cycle count, such as asserting [done] for just one cycle after a write, must be
/// enforced and carried out by the interpreter. This register enforces no rules about
/// when its state can be modified.
#[derive(Clone, Debug)]
pub struct StdReg {
    pub width: u64,
    pub val: Value,
}

impl StdReg {
    /// New registers have unitialized values -- only specify their widths
    pub fn new(width: u64) -> StdReg {
        StdReg {
            width,
            val: Value::new(width as usize),
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

// fn execute_mut<'a>(
//     &mut self,
//     inputs: &'a [(ir::Id, Value)],
// ) -> Vec<(ir::Id, OutputValue)>;

impl ExecuteStateful for StdReg {
    fn execute_mut(
        //have to put lifetimes
        &mut self,
        inputs: &[(ir::Id, Value)],
    ) -> Vec<(ir::Id, OutputValue)> {
        //unwrap the arguments
        let (_, input) = inputs.iter().find(|(id, _)| id == "in").unwrap();
        let (_, write_en) =
            inputs.iter().find(|(id, _)| id == "write_en").unwrap();
        //make sure [input] isn't too wide
        check_width(input, self.width);
        //write the input to the register
        if write_en.as_u64() == 1 {
            let old = self.val.clone();
            self.val = input.clone();
            // what's in this vector:
            // the "out" -- TimeLockedValue ofthe new register data. Needs 1 cycle before readable
            // "done" -- TimeLockedValue of DONE, which is asserted 1 cycle after we write
            // all this coordination is done by the interpreter. We just set it up correctly
            vec![
                (
                    ir::Id::from("out"),
                    TimeLockedValue::new(self.val.clone(), 1, Some(old)).into(),
                ),
                (
                    ir::Id::from("done"),
                    TimeLockedValue::new(
                        Value::try_from_init(1, 1).unwrap(),
                        1,
                        Some(Value::zeroes(1)),
                    )
                    .into(),
                ),
            ]
        } else {
            // if write_en was low, so done is 0 b/c nothing was written here
            // in this vector i
            // OUT: the old value in the register, b/c we couldn't write
            // DONE: not TimeLockedValue, b/c it's just 0, b/c our write was unsuccessful
            vec![
                (ir::Id::from("out"), self.val.clone().into()),
                (ir::Id::from("done"), Value::zeroes(1).into()),
            ]
        }
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
        StdConst { width, val: val }
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
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        check_widths(left, right, self.width);
        let mut tr = left.vec.clone();
        tr.shift_right(right.as_u64() as usize);
        Value { vec: tr }.into()
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
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> OutputValue {
        check_widths(left, right, self.width);
        let mut tr = left.vec.clone();
        tr.shift_left(right.as_u64() as usize);
        Value { vec: tr }.into()
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
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 + right_64;
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val, bitwidth).into()
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
    /// let val_3_4bit = std_sub_4_bit.execute_bin(&val_4_4bit, &val_1_4bit);
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
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        // Bitwise subtraction: left - right = left + (!right + 1)
        let right = Value {
            vec: !(right.vec.clone()),
        };
        let right_64 = right.as_u64() + 1;
        //need to allow overflow -- we are dealing only with bits
        let init_val = left_64 + right_64;
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val, bitwidth).into()
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
        check_widths(input, input, self.in_width);
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
        check_widths(input, input, self.in_width);
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
        check_widths(input, input, self.width);
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
        check_widths(left, right, self.width);
        Value {
            vec: left.vec.clone() & right.vec.clone(),
        }
        .into()
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
        check_widths(left, right, self.width);
        Value {
            vec: left.vec.clone() | right.vec.clone(),
        }
        .into()
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
        check_widths(left, right, self.width);
        Value {
            vec: left.vec.clone() ^ right.vec.clone(),
        }
        .into()
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
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 > right_64;

        Value::from_init(init_val, 1 as usize).into()
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
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 < right_64;

        Value::from_init(init_val, 1 as usize).into()
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
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 == right_64;

        Value::from_init(init_val, 1 as usize).into()
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
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 != right_64;
        Value::from_init(init_val, 1 as usize).into()
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
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 >= right_64;

        Value::from_init(init_val, 1 as usize).into()
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
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 <= right_64;
        Value::from_init(init_val, 1 as usize).into()
    }
}

// Uses the cell's inputs ports to perform any required updates to the
// cell's output ports.
// TODO: how to get input and output ports in general? How to "standardize" for combinational or not operations
// pub fn update_cell_state(
//     cell: &ir::Id,
//     inputs: &[ir::Id],
//     output: &[ir::Id],
//     env: &Environment, // should this be a reference
//     component: ir::Id,
// ) -> FutilResult<Environment> {
//     // get the actual cell, based on the id
//     // let cell_r = cell.as_ref();

//     let mut new_env = env.clone();

//     let cell_r = new_env
//         .get_cell(&component, cell)
//         .unwrap_or_else(|| panic!("Cannot find cell with name"));

//     let temp = cell_r.borrow();

//     // get the cell type
//     let cell_type = temp.type_name().unwrap_or_else(|| panic!("Futil Const?"));

//     match cell_type.id.as_str() {
//         "std_reg" => {
//             // TODO: this is wrong...
//             let write_en = ir::Id::from("write_en");

//             // register's write_en must be high to write reg.out and reg.done
//             if new_env.get(&component, &cell, &write_en).as_u64() != 0 {
//                 let out = ir::Id::from("out"); //assuming reg.in = cell.out, always
//                 let inp = ir::Id::from("in"); //assuming reg.in = cell.out, always
//                 let done = ir::Id::from("done"); //done id

//                 new_env.put(
//                     &component,
//                     cell,
//                     &output[0],
//                     env.get(&component, &inputs[0], &out),
//                 ); //reg.in = cell.out; should this be in init?

//                 if output[0].id == "in" {
//                     new_env.put(
//                         &component,
//                         cell,
//                         &out,
//                         new_env.get(&component, cell, &inp),
//                     ); // reg.out = reg.in
//                     new_env.put(
//                         &component,
//                         cell,
//                         &done,
//                         Value::try_from_init(1, 1).unwrap(),
//                     ); // reg.done = 1'd1
//                        //new_env.remove_update(cell); // remove from update queue
//                 }
//             }
//         }
//         "std_mem_d1" => {
//             let mut mem = HashMap::new();
//             let out = ir::Id::from("out");
//             let write_en = ir::Id::from("write_en");
//             let done = ir::Id::from("done"); //done id

//             // memory should write to addres
//             if new_env.get(&component, &cell, &write_en).as_u64() != 0 {
//                 let addr0 = ir::Id::from("addr0");
//                 let _read_data = ir::Id::from("read_data");
//                 let write_data = ir::Id::from("write_data");

//                 new_env.put(
//                     &component,
//                     cell,
//                     &output[0],
//                     env.get(&component, &inputs[0], &out),
//                 );

//                 let data = new_env.get(&component, cell, &write_data);
//                 mem.insert(addr0, data);
//             }
//             // read data
//             if output[0].id == "read_data" {
//                 let addr0 = ir::Id::from("addr0");

//                 let dat = match mem.get(&addr0) {
//                     Some(&num) => num,
//                     _ => panic!("nothing in the memory"),
//                 };

//                 new_env.put(&component, cell, &output[0], dat);
//             }
//             new_env.put(
//                 &component,
//                 cell,
//                 &done,
//                 Value::try_from_init(1, 1).unwrap(),
//             );
//         }
//         "std_sqrt" => {
//             //TODO; wrong implementation
//             // new_env.put(
//             //     cell,
//             //     &output[0],
//             //     ((new_env.get(cell, &inputs[0]) as f64).sqrt()) as u64, // cast to f64 to use sqrt
//             // );
//         }
//         "std_add" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             new_env.get(&component, cell, &inputs[0])
//                 + env.get(&component, cell, &inputs[1]),
//         ),
//         "std_sub" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             new_env.get(&component, cell, &inputs[0])
//                 - env.get(&component, cell, &inputs[1]),
//         ),
//         "std_mod" => {
//             if env.get(&component, cell, &inputs[1]).as_u64() != 0 {
//                 new_env.put(
//                     &component,
//                     cell,
//                     &output[0],
//                     new_env.get(&component, cell, &inputs[0])
//                         % env.get(&component, cell, &inputs[1]),
//                 )
//             }
//         }
//         "std_mult" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             new_env.get(&component, cell, &inputs[0])
//                 * env.get(&component, cell, &inputs[1]),
//         ),
//         "std_div" => {
//             // need this condition to avoid divide by 0
//             // (e.g. if only one of left/right ports has been updated from the initial nonzero value?)
//             // TODO: what if the program specifies a divide by 0? how to catch??
//             if env.get(&component, cell, &inputs[1]).a != 0 {
//                 new_env.put(
//                     &component,
//                     cell,
//                     &output[0],
//                     new_env.get(&component, cell, &inputs[0])
//                         / env.get(&component, cell, &inputs[1]),
//                 )
//             }
//         }
//         "std_not" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             !new_env.get(&component, cell, &inputs[0]),
//         ),
//         "std_and" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             new_env.get(&component, cell, &inputs[0])
//                 & env.get(&component, cell, &inputs[1]),
//         ),
//         "std_or" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             new_env.get(&component, cell, &inputs[0])
//                 | env.get(&component, cell, &inputs[1]),
//         ),
//         "std_xor" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             new_env.get(&component, cell, &inputs[0])
//                 ^ env.get(&component, cell, &inputs[1]),
//         ),
//         "std_gt" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             (new_env.get(&component, cell, &inputs[0])
//                 > env.get(&component, cell, &inputs[1])) as u64,
//         ),
//         "std_lt" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             (new_env.get(&component, cell, &inputs[0])
//                 < env.get(&component, cell, &inputs[1])) as u64,
//         ),
//         "std_eq" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             (new_env.get(&component, cell, &inputs[0])
//                 == env.get(&component, cell, &inputs[1])) as u64,
//         ),
//         "std_neq" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             (new_env.get(&component, cell, &inputs[0])
//                 != env.get(&component, cell, &inputs[1])) as u64,
//         ),
//         "std_ge" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             (new_env.get(&component, cell, &inputs[0])
//                 >= env.get(&component, cell, &inputs[1])) as u64,
//         ),
//         "std_le" => new_env.put(
//             &component,
//             cell,
//             &output[0],
//             (new_env.get(&component, cell, &inputs[0])
//                 <= env.get(&component, cell, &inputs[1])) as u64,
//         ),
//         _ => unimplemented!("{}", cell_type),
//     }

//     // TODO
//     Ok(new_env)
// }
