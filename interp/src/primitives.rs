//! Defines update methods for the various primitive cells in the Calyx
// standard library.

use super::environment::Environment;
use super::values::Value;
use calyx::{errors::FutilResult, ir};
use std::collections::HashMap;
use std::convert::TryInto;
use std::ops::*;

pub enum Primitve {
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
}

/// For binary operator components that taken in a <left> Value and
/// <right> Value.
///
/// # Example
/// ```
/// let std_add = StdAdd::new(5) // A 5 bit adder
/// let one_plus_two = std_add.execute_bin(
///     &(Value::try_from_init(1, 5).unwrap()),
///     &(Value::try_from_init(2, 5).unwrap())
/// )
/// ```
pub trait ExecuteBinary {
    fn execute_bin(&self, left: &Value, right: &Value) -> Value;
}

/// Only binary operator components have trait [Execute].
pub trait Execute {
    fn execute<'a>(
        &self,
        inputs: &'a [(ir::Id, Value)],
        outputs: &'a [(ir::Id, Value)],
    ) -> Vec<(ir::Id, Value)>;
}

/// For unary operator components that only take in one input.
/// # Example
/// ```
///let std_not = StdNot::new(5) // a 5 bit not-er
/// let not_one = std_not.execute_unary(&(Value::try_from_init(1, 5).unwrap()));
/// ```
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

/// Ensures the input values are of the appropriate widths, else panics.
fn check_widths(left: &Value, right: &Value, width: u64) -> () {
    if width != (left.vec.len() as u64)
        || width != (right.vec.len() as u64)
        || left.vec.len() != right.vec.len()
    {
        panic!("Width mismatch between the component and the value.");
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
pub struct StdMemD1 {
    pub width: u64,    // size of individual piece of mem
    pub size: u64,     // # slots of mem
    pub idx_size: u64, // # bits needed to index a piece of mem
    pub data: Vec<Value>,
    pub write_en: bool,
}

impl StdMemD1 {
    pub fn new(width: u64, size: u64, idx_size: u64) -> StdMemD1 {
        let data = vec![
            Value::zeroes((width as usize).try_into().unwrap());
            (size as usize).try_into().unwrap()
        ];
        StdMemD1 {
            width,
            size,
            idx_size,
            data,
            write_en: false,
        }
    }
}
//std_memd2 :
pub struct StdMemD2 {}

impl StdMemD2 {}

//std_memd3 :
pub struct StdMemD3 {}

impl StdMemD3 {}

//std_memd4 :
pub struct StdMemD4 {}

impl StdMemD4 {}

/// A Standard Register of a certain [width].
/// Rules regarding cycle count, such as asserting [done] for just one cycle after a write, must be
/// enforced and carried out by the interpreter. This register only ensures no writes
/// occur while [write_en] is low.
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

    /// Sets value in register, only if [write_en] is high.
    /// # Example
    /// ```
    /// let reg_16 = StdReg::new(16);
    /// let val_6_16bit = Value::try_from_init(6, 16).unwrap();
    /// reg_16.load_value(val_6_16bit);
    /// ```
    /// # Panic
    /// * panics if width of [input] != self.width
    pub fn load_value(&mut self, input: Value) {
        check_widths(&input, &input, self.width);
        if self.write_en {
            self.val = input.truncate(self.width.try_into().unwrap())
        }
    }

    /// After loading a value into the register, use [set_done_high] to emit
    /// the done signal. Note that the [StdReg] struct has no sense of time
    /// itself. The interpreter is responsible for setting the [done] signal
    /// high for exactly one cycle.
    pub fn set_done_high(&mut self) {
        self.done = true
    }
    /// Pairs with [set_done_high].
    pub fn set_done_low(&mut self) {
        self.done = false
    }

    /// A cycle before trying to load a value into the register, make sure to
    /// [set_write_en_high].
    pub fn set_write_en_high(&mut self) {
        self.write_en = true
    }

    pub fn set_write_en_low(&mut self) {
        self.write_en = false
    }

    /// Reads the value from the register. Makes no guarantee on the validity
    /// of data in the register -- the interpreter must check [done] itself.
    pub fn read_value(&self) -> Value {
        self.val.clone()
    }

    pub fn read_u64(&self) -> u64 {
        self.val.as_u64()
    }
}

/// A component that keeps one value, that can't be rewritten. Is immutable,
/// and instantiated with the value it holds, which must have the same # of bits as [width].
pub struct StdConst {
    width: u64,
    val: Value,
}

impl StdConst {
    /// Instantiates a new constant component
    /// # Example
    /// ```
    /// let const_16bit_9 = StdConst::new(16, 9);
    /// ```
    ///
    /// # Panics
    /// * Panics if [val] != [width]
    pub fn new(width: u64, val: Value) -> StdConst {
        check_widths(&val, &val, width);
        StdConst { width, val: val }
    }

    /// Returns the value this constant component represents
    /// # Example
    /// ```
    /// let const_16bit_9 = StdConst::new(16, 9);
    /// let val_9 = const_16bit_9.read_value();
    /// ```
    pub fn read_val(&self) -> Value {
        self.val.clone()
    }

    /// Returns the u64 corresponding to the value this constant component represents
    /// # Example
    /// ```
    /// let const_16bit_9 = StdConst::new(16, 9);
    /// assert_eq!(const_16bit_9.as_u64(), 9);
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

pub struct StdLsh {
    width: u64,
}

impl StdLsh {
    /// Instantiate a new StdLsh of a specific width
    /// # Example
    /// ```
    /// let std_lsh_16_bit = StdLsh::new(16)
    /// ```
    pub fn new(width: u64) -> StdLsh {
        StdLsh { width }
    }
}

impl ExecuteBinary for StdLsh {
    /// Returns the Value representing LEFT << RIGHT
    /// # Example
    /// ```
    /// let std_lsh_16_bit = StdLsh::new(16);
    /// let val_2_16bit = Value::try_from_init(2, 16).unwrap();
    /// let val_8_16bit = std_lsh_16_bit.execute_bin(&val_2_16bit, &val_2_16bit);
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let mut tr = left.vec.clone();
        tr.shift_right(right.as_u64() as usize);
        Value { vec: tr }
    }
}

/// std_rsh<WIDTH>
/// A right bit shift. Performs LEFT >> RIGHT. This component is combinational.

/// Inputs:

/// left: WIDTH - A WIDTH-bit value to be shifted
/// right: WIDTH - A WIDTH-bit value representing the shift amount
/// Outputs:

/// out: WIDTH - A WIDTH-bit value equivalent to LEFT >> RIGHT

pub struct StdRsh {
    width: u64,
}

impl StdRsh {
    /// Instantiate a new StdRsh component with a specified width
    /// # Example
    /// ```
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
    /// let std_rsh_16_bit = StdRsh::new(16);
    /// let val_8_16bit = Value::try_from_init(8, 16).unwrap();
    /// let val_1_16bit = Value::try_from_init(1, 16).unwrap();
    /// let val_4_16bit = std_rsh_16_bit.execute_bin(&val_8_16bit, &val_1_16bit);
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let mut tr = left.vec.clone();
        tr.shift_left(right.as_u64() as usize);
        Value { vec: tr }
    }
}

//std_add<WIDTH>
//Bitwise addition without a carry flag. Performs LEFT + RIGHT. This component is combinational.
//Inputs:
//left: WIDTH - A WIDTH-bit value
//right: WIDTH - A WIDTH-bit value
//Outputs:
//out: WIDTH - A WIDTH-bit value equivalent to LEFT + RIGHT
pub struct StdAdd {
    width: u64,
}

impl StdAdd {
    /// Instantiate a new StdAdd with a specified bit width
    /// # Example
    /// ```
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
    /// let std_add_16bit = StdAdd::new(16);
    /// let val_8_16bit = Value::try_from_init(8, 16).unwrap();
    /// let val_1_16bit = Value::try_from_init(1, 16).unwrap();
    /// let val_9_16bit = std_add_16bit.execute_bin(&val_8_16bit, &val_2_1bit);
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 + right_64;
        let bitwidth: usize = left.vec.len();
        Value::from_init(init_val, bitwidth)
    }
}

/// std_sub<WIDTH>
/// Bitwise subtraction. Performs LEFT - RIGHT. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit value
/// right: WIDTH - A WIDTH-bit value
/// Outputs:
/// out: WIDTH - A WIDTH-bit value equivalent to LEFT - RIGHT

pub struct StdSub {
    width: u64,
}

impl StdSub {
    /// Instantiates a new standard subtraction component
    /// # Example
    /// ```
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
    /// //4 [0100] - 1 [0001] = 3 [0011]
    /// let val_4_4bit = Value::try_from_init(4, 4).unwrap();
    /// let val_1_4bit = Value::try_from_init(1, 4).unwrap();
    /// let std_sub_4_bit = StdSub::new(4);
    /// let val_3_4bit = std_sub_4_bit.execute_bin(&val_4_4bit, &val_1_4bit);
    /// //4 [0100] - 5 [0101] = -1 [1111] <- as an unsigned binary num, this is 15
    /// let val_5_4bit = Value::try_from_init(5, 4).unwrap();
    /// assert_eq!(std_sub_4_bit.execute_bin(&val_4_4bit, &val_5_4bit).as_u64(), 15);
    /// ```
    ///
    /// # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
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
        Value::from_init(init_val, bitwidth)
    }
}

/// Slice out the lower OUT_WIDTH bits of an IN_WIDTH-bit value. Computes
/// in[out_width - 1 : 0]. This component is combinational.
/// Inputs:
/// in: IN_WIDTH - An IN_WIDTH-bit value
/// Outputs:
/// out: OUT_WIDTH - The lower (from LSB towards MSB) OUT_WIDTH bits of in
pub struct StdSlice {
    in_width: u64,
    out_width: u64,
}

impl StdSlice {
    /// Instantiate a new instance of StdSlice
    ///
    /// # Example
    /// ```
    /// let std_slice_6_to_4 = StdSlice::new(6, 4)
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
    /// let val_5_3bits = Value::try_from_init(5, 3); // 5 = [101]
    /// let std_slice_3_to_2 = StdSlice::new(3, 2);
    /// let val_1_2bits = std_slice_1_to_2.execute_unary(val_5_3bits) // 1 = [01]
    /// ```
    ///
    /// # Panics
    /// * panics if input's width and self.width are not equal
    ///
    fn execute_unary(&self, input: &Value) -> Value {
        check_widths(input, input, self.in_width);
        let tr = input.clone();
        tr.truncate(self.out_width as usize)
    }
}

/// Given an IN_WIDTH-bit input, zero pad from the MSB to an output of
/// OUT_WIDTH-bits. This component is combinational.
/// Inputs:
/// in: IN_WIDTH - An IN_WIDTH-bit value to be padded
/// Outputs:
/// out: OUT_WIDTH - The paddwd width
pub struct StdPad {
    in_width: u64,
    out_width: u64,
}

impl StdPad {
    /// Instantiate instance of StdPad that takes input with width [in_width] and returns output with width [out_width]
    /// # Example
    /// ```
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
    /// let val_5_3bits = Value::try_from_init(5, 3); // 5 = [101]
    /// let std_pad_3_to_5 = StdPad::new(3, 5);
    /// let val_5_5bits = std_pad_3_to_5.execute_unary(val_5_3bits) // 5 = [00101]
    /// ```
    ///
    /// # Panics
    /// * panics if input's width and self.width are not equal
    ///
    fn execute_unary(&self, input: &Value) -> Value {
        check_widths(input, input, self.in_width);
        let pd = input.clone();
        pd.ext(self.out_width as usize)
    }
}

/* =========================== Logical Operators =========================== */
/// std_not<WIDTH>
/// Bitwise NOT. This component is combinational.
/// Inputs:
/// in: WIDTH - A WIDTH-bit input.
/// Outputs:
/// out: WIDTH - The bitwise NOT of the input (~in)
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
    /// let val_5_3bits = Value::try_from_init(5, 3); // 5 = [101]
    /// let std_not_3bit = StdNot::new(3);
    /// let val_2_3bits = std_not_3bits.execute_unary(val_5_3bits);
    /// assert_eq!(val_2_3bits.as_u64(), 2)
    /// ```
    ///
    /// # Panics
    /// * panics if input's width and self.width are not equal
    ///
    fn execute_unary<'a>(&self, input: &Value) -> Value {
        check_widths(input, input, self.width);
        Value {
            vec: input.vec.clone().not(),
        }
    }
}

/// std_and<WIDTH>
/// Bitwise AND. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:

// out: WIDTH - The bitwise AND of the arguments (left & right)
pub struct StdAnd {
    width: u64,
}

impl StdAnd {
    /// Instantiate an instance of StdAnd that accepts input of width [width]
    /// # Example
    /// ```
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
    /// let val_5_3 = Value::try_from_init(5, 3).unwrap();
    /// let val_2_3 = Value::try_from_init(2, 3).unwrap();
    /// let std_and_3bit = StdAdd::new(3);
    /// let val_0_3 = std_and_3bit.execute_bin(&val_5_3, &val_2_3);
    /// assert_eq!(val_0_3.as_u64(), 0);
    /// ```
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        Value {
            vec: left.vec.clone() & right.vec.clone(),
        }
    }
}

/// std_or<WIDTH>
/// Bitwise OR. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: WIDTH - The bitwise OR of the arguments (left | right)
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
    /// let val_5_3 = Value::try_from_init(5, 3).unwrap(); // 5 = [101]
    /// let val_2_3 = Value::try_from_init(2, 3).unwrap(); // 2 = [010]
    /// let std_or_3bit = StOr::new(3);
    /// let val_7_3 = std_or_3bit.execute_bin(&val_5_3, &val_2_3);
    /// assert_eq!(val_7_3.as_u64(), 7);
    /// ```
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        Value {
            vec: left.vec.clone() | right.vec.clone(),
        }
    }
}

/// std_xor<WIDTH>
/// Bitwise XOR. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: WIDTH - The bitwise XOR of the arguments (left ^ right)
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
    /// let val_7_3 = Value::try_from_init(7, 3).unwrap(); // 7 = [111]
    /// let val_2_3 = Value::try_from_init(2, 3).unwrap(); // 2 = [010]
    /// let std_xor_3bit = StXor::new(3);
    /// let val_5_3 = std_xor_3bit.execute_bin(&val_7_3, &val_2_3);
    /// assert_eq!(val_5_3.as_u64(), 5);
    /// ```
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        Value {
            vec: left.vec.clone() ^ right.vec.clone(),
        }
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
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_gt_3bit = StdGt::new(3);
    /// assert_eq!(std_gt_3bit.execute_bin(&val_2_3.bit, &val_1_3bit).as_u64(), 1);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 > right_64;

        Value::from_init(init_val, 1 as usize)
    }
}

/// std_lt<WIDTH>
/// Less than. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: 1 - A single bit output. 1 if left < right else 0.
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
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_lt_3bit = StdLt::new(3);
    /// assert_eq!(std_lt_3bit.execute_bin(&val_2_3.bit, &val_1_3bit).as_u64(), 0);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 < right_64;

        Value::from_init(init_val, 1 as usize)
    }
}

/// std_eq<WIDTH>
/// Equality comparison. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: 1 - A single bit output. 1 if left = right else 0.
pub struct StdEq {
    width: u64,
}

impl StdEq {
    /// Instantiates a StdEq that only accepts inputs of width [width]
    /// # Example
    /// ```
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
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_eq_3bit = StdEq::new(3);
    /// assert_eq!(std_eq_3bit.execute_bin(&val_2_3.bit, &val_1_3bit).as_u64(), 0);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 == right_64;

        Value::from_init(init_val, 1 as usize)
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
pub struct StdNeq {
    width: u64,
}

impl StdNeq {
    /// Instantiates a StdNeq component that only accepts inputs of width [width]
    /// /// # Example
    /// ```
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
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_neq_3bit = StdNeq::new(3);
    /// assert_eq!(std_neq_3bit.execute_bin(&val_2_3.bit, &val_1_3bit).as_u64(), 1);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 != right_64;
        Value::from_init(init_val, 1 as usize)
    }
}

/// std_ge<WIDTH>
/// Greater than or equal. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: 1 - A single bit output. 1 if left >= right else 0.
pub struct StdGe {
    width: u64,
}
impl StdGe {
    /// Instantiate a new StdGe component that accepts only inputs of width [width]
    /// /// # Example
    /// ```
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
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_ge_3bit = StdGe::new(3);
    /// assert_eq!(std_ge_3bit.execute_bin(&val_2_3.bit, &val_1_3bit).as_u64(), 1);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 >= right_64;

        Value::from_init(init_val, 1 as usize)
    }
}

/// std_le<WIDTH>
/// Less than or equal. This component is combinational.
/// Inputs:
/// left: WIDTH - A WIDTH-bit argument
/// right: WIDTH - A WIDTH-bit argument
/// Outputs:
/// out: 1 - A single bit output. 1 if left <= right else 0.
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
    /// let val_2_3bit = Value::try_from_init(2, 3).unwrap();
    /// let val_1_3bit = Value::try_from_init(1, 3).unwrap();
    /// let std_lt_3bit = StdLt::new(3);
    /// assert_eq!(std_lt_3bit.execute_bin(&val_2_3.bit, &val_1_3bit).as_u64(), 0);
    /// ```
    ///  # Panics
    /// * panics if left's width, right's width and self.width are not all equal
    ///
    fn execute_bin(&self, left: &Value, right: &Value) -> Value {
        check_widths(left, right, self.width);
        let left_64 = left.as_u64();
        let right_64 = right.as_u64();
        let init_val = left_64 <= right_64;
        Value::from_init(init_val, 1 as usize)
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
