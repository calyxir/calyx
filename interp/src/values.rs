use bitvec::prelude::*;
use serde::de::{self, Deserialize, Visitor};

// Lsb0 means [10010] gives 0 at index 0, 1 at index 1, 0 at index 2, etc
// from documentation, usize is the best data type to use in bitvec.
#[derive(Debug)]
pub struct ValueError {}

pub enum InputNumber {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Usize(usize),
}

impl From<u8> for InputNumber {
    fn from(i: u8) -> Self {
        Self::U8(i)
    }
}
impl From<u16> for InputNumber {
    fn from(i: u16) -> Self {
        Self::U16(i)
    }
}
impl From<u32> for InputNumber {
    fn from(i: u32) -> Self {
        Self::U32(i)
    }
}
impl From<u64> for InputNumber {
    fn from(i: u64) -> Self {
        Self::U64(i)
    }
}
impl From<u128> for InputNumber {
    fn from(i: u128) -> Self {
        Self::U128(i)
    }
}
impl From<i8> for InputNumber {
    fn from(i: i8) -> Self {
        Self::I8(i)
    }
}
impl From<i16> for InputNumber {
    fn from(i: i16) -> Self {
        Self::I16(i)
    }
}
impl From<i32> for InputNumber {
    fn from(i: i32) -> Self {
        Self::I32(i)
    }
}
impl From<i64> for InputNumber {
    fn from(i: i64) -> Self {
        Self::I64(i)
    }
}
impl From<i128> for InputNumber {
    fn from(i: i128) -> Self {
        Self::I128(i)
    }
}
impl From<usize> for InputNumber {
    fn from(i: usize) -> Self {
        Self::Usize(i)
    }
}

impl InputNumber {
    fn as_usize(&self) -> usize {
        match self {
            InputNumber::U8(i) => *i as usize,
            InputNumber::U16(i) => *i as usize,
            InputNumber::U32(i) => *i as usize,
            InputNumber::U64(i) => *i as usize,
            InputNumber::U128(i) => *i as usize,
            InputNumber::I8(i) => *i as usize,
            InputNumber::I16(i) => *i as usize,
            InputNumber::I32(i) => *i as usize,
            InputNumber::I64(i) => *i as usize,
            InputNumber::I128(i) => *i as usize,
            InputNumber::Usize(i) => *i,
        }
    }
    fn as_bit_vec(&self) -> BitVec<Lsb0, u64> {
        match self {
            InputNumber::U8(i) => BitVec::from_element(*i as u64),
            InputNumber::U16(i) => BitVec::from_element(*i as u64),
            InputNumber::U32(i) => BitVec::from_element(*i as u64),
            InputNumber::U64(i) => BitVec::from_element(*i),
            InputNumber::U128(i) => {
                let lower = (i & (u64::MAX as u128)) as u64;
                let upper = ((i >> 64) & u64::MAX as u128) as u64;
                BitVec::from_slice(&[lower, upper]).unwrap()
            }
            InputNumber::I8(i) => BitVec::from_element(*i as u64),
            InputNumber::I16(i) => BitVec::from_element(*i as u64),
            InputNumber::I32(i) => BitVec::from_element(*i as u64),
            InputNumber::I64(i) => BitVec::from_element(*i as u64),
            InputNumber::I128(i) => {
                let lower = (i & (u64::MAX as i128)) as u64;
                let upper = ((i >> 64) & u64::MAX as i128) as u64;
                BitVec::from_slice(&[lower, upper]).unwrap()
            }
            InputNumber::Usize(i) => BitVec::from_element(*i as u64),
        }
    }
}
#[derive(Clone, Debug, Default)]
/// The type of all inputs and outputs to all components in Calyx.
/// Wraps a BitVector.
pub struct Value {
    // Lsb0 means the 0th index contains the LSB. This is useful because
    // a 7-bit bitvector and 17-bit bitvector representing the number 6 have
    // ones in the same index.
    pub vec: BitVec<Lsb0, u64>,
}

impl Value {
    pub fn unsigned_value_fits_in(&self, width: usize) -> bool {
        self.vec.len() <= width // obviously fits then
            || self
                .vec
                .last_one() // returns an index
                .map(|x| x < width)
                .unwrap_or(true) // if there is no high bit then it can fit in the given width
    }

    pub fn signed_value_fits_in(&self, width: usize) -> bool {
        self.vec.len() <= width // obviously fits then
        || (self.vec.ends_with(bits![0]) && self.unsigned_value_fits_in(width - 1)) // positive value (technically wastes a check)
        || (self.vec.ends_with(bits![1]) && ((self.vec.len() - self.vec.trailing_ones()) < width) || self.vec.trailing_ones() == 0)
        // negative value greater than or equal to lowest in new width
    }

    pub fn width(&self) -> u64 {
        self.vec.len() as u64
    }
    /// Creates a Value with the specified bandwidth.
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let empty_val = Value::new(2 as usize);
    /// ```
    pub fn new(bitwidth: usize) -> Value {
        Value::zeroes(bitwidth)
    }

    /// Creates a new Value initialized to all 0s given a bitwidth.
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let zeroed_val = Value::zeroes(2 as usize);
    /// ```
    pub fn zeroes<I: Into<InputNumber>>(bitwidth: I) -> Value {
        let input_num: InputNumber = bitwidth.into();
        Value {
            vec: bitvec![Lsb0, u64; 0; input_num.as_usize()],
        }
    }

    pub fn bit_high() -> Value {
        Value::from(1_u64, 1_usize)
    }

    pub fn bit_low() -> Value {
        Value::from(0_u64, 1_usize)
    }

    /// Create a new Value of a given bitwidth out of an initial_val. You do
    /// not have to guarantee initial_val satisifies Into<u64>. Note: will error if the
    /// given width cannot be made into a usize.
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let val_16_16 = Value::from(16, 16);
    /// ```
    pub fn from<T1: Into<InputNumber>, T2: Into<InputNumber>>(
        initial_val: T1,
        bitwidth: T2,
    ) -> Self {
        let init: InputNumber = initial_val.into();
        let mut bv_init = init.as_bit_vec();
        let width: InputNumber = bitwidth.into();
        bv_init.resize(width.as_usize(), false);
        Value { vec: bv_init }
    }

    /// Returns a Value containing a vector of length 0, effectively returning
    /// a cleared vector.
    pub fn clear(&self) -> Self {
        let mut vec = self.vec.clone();
        vec.truncate(0);
        Value { vec }
    }

    /// Returns a Value truncated to length [new_size].
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let val_4_4 = Value::from(4, 16).truncate(4);
    /// ```
    pub fn truncate(&self, new_size: usize) -> Value {
        let mut vec = self.vec.clone();
        vec.truncate(new_size);
        Value { vec }
    }

    /// Zero-extend the vector to length [ext].
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let val_4_16 = Value::from(4, 4).ext(16);
    /// ```
    pub fn ext(&self, ext: usize) -> Value {
        let mut vec = self.vec.clone();
        for _x in 0..(ext - vec.len()) {
            vec.push(false);
        }
        Value { vec }
    }

    /// Sign-extend the vector to length [ext].
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// // [1111] -> [11111]. In 2'sC these are both -1
    /// let val_31_5 = Value::from(15, 4).sext(5);
    /// ```
    pub fn sext(&self, ext: usize) -> Value {
        let mut vec = self.vec.clone();
        let sign = vec[vec.len() - 1];
        for _x in 0..(ext - vec.len()) {
            vec.push(sign);
        }
        Value { vec }
    }

    /// Converts value into u64 type. Vector within Value can be of any width. The value
    /// will be truncated to fit the specified width if it exceeds it
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let unsign_64_16 = Value::from(16, 16).as_u64();
    /// ```
    pub fn as_u64(&self) -> u64 {
        assert!(
            self.unsigned_value_fits_in(64),
            "Cannot fit value into an u64"
        );
        self.vec
            .iter()
            .enumerate()
            .take(64)
            .fold(0_u64, |acc, (idx, bit)| -> u64 {
                acc | ((*bit as u64) << idx)
            })
    }

    /// Converts value into u128 type. Vector within Value can be of any width. The
    /// value will be truncated if it exceeds 128 bits
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let unsign_128 = Value::from(u128::MAX - 2, 128).as_u128();
    /// assert_eq!(unsign_128, u128::MAX - 2);
    /// let unsign_128_32 = Value::from(u128::MAX - 4, 32).as_u128();
    /// assert_eq!(unsign_128_32, ((u128::MAX - 4) as u32) as u128);
    /// ```
    pub fn as_u128(&self) -> u128 {
        assert!(
            self.unsigned_value_fits_in(128),
            "Cannot fit value into an u128"
        );
        self.vec
            .iter()
            .enumerate()
            .take(128)
            .fold(0_u128, |acc, (idx, bit)| -> u128 {
                acc | ((*bit as u128) << idx)
            })
    }

    /// Converts value into i64 type using 2C representation. Truncates to 64 bits if
    /// the value exceeds 64 bits. Sign extends lower values
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let signed_neg_1_4 = Value::from(15, 4).as_i64();
    /// assert_eq!(signed_neg_1_4, -1);
    /// ```
    pub fn as_i64(&self) -> i64 {
        assert!(
            self.signed_value_fits_in(64),
            "Cannot fit value into an i64"
        );
        let init = if *(&self.vec).last().unwrap() { -1 } else { 0 };
        self.vec.iter().enumerate().take(64).fold(
            init,
            |acc, (idx, bit)| -> i64 {
                (acc & (!(1 << idx))) | ((*bit as i64) << idx)
            },
        )
    }

    /// Converts value into i128 type using 2C representation. Truncates to 128 bits if
    /// the value exceeds 128 bits. Sign extends lower values
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let signed_neg_1_4 = Value::from(-1_i128, 4).as_i128();
    /// assert_eq!(signed_neg_1_4, -1);
    /// let signed_pos = Value::from(5_i128,10).as_i128();
    /// assert_eq!(signed_pos, 5)
    /// ```
    pub fn as_i128(&self) -> i128 {
        assert!(
            self.signed_value_fits_in(128),
            "Cannot fit value into an i128"
        );
        let init = if *(&self.vec).last().unwrap() { -1 } else { 0 };
        self.vec.iter().enumerate().take(128).fold(
            init,
            |acc, (idx, bit)| -> i128 {
                (acc & (!(1 << idx))) | ((*bit as i128) << idx)
            },
        )
    }

    pub fn as_usize(&self) -> usize {
        assert!(
            self.unsigned_value_fits_in(usize::BITS as usize),
            "Cannot fit value into an usize"
        );

        self.vec
            .iter()
            .enumerate()
            .take(usize::BITS as usize)
            .fold(0_usize, |acc, (idx, bit)| -> usize {
                acc | ((*bit as usize) << idx)
            })
    }

    pub fn as_bool(&self) -> bool {
        assert!(self.vec.len() == 1);
        self.vec[0]
    }

    #[allow(clippy::len_without_is_empty)]
    /// Returns the length (bitwidth) of the value
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let v = Value::from(1, 3);
    /// assert_eq!(v.len(), 3)
    /// ```
    pub fn len(&self) -> usize {
        self.vec.len()
    }
}

/* ============== Impls for Values to make them easier to use ============= */
#[allow(clippy::from_over_into)]
impl Into<u64> for Value {
    fn into(self) -> u64 {
        self.as_u64()
    }
}

impl std::fmt::Display for Value {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        let mut vec_rev = self.vec.clone();
        vec_rev.reverse();
        write!(f, "{}", vec_rev)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.vec.len() == other.vec.len() && self.vec == other.vec
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    /// Unsigned ordering comparison
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        assert!(self.vec.len() == other.vec.len());
        for (us_bit, them_bit) in self
            .vec
            .iter()
            .by_ref()
            .rev()
            .zip(other.vec.iter().by_ref().rev())
        {
            match (us_bit, them_bit) {
                (true, true) | (false, false) => {} // so far equal
                (true, false) => return Some(std::cmp::Ordering::Greater),
                (false, true) => return Some(std::cmp::Ordering::Less),
            };
        }
        Some(std::cmp::Ordering::Equal)
    }
}

pub trait ReadableValue {
    fn get_val(&self) -> &Value;
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BitVecVisitor;

        impl<'de> Visitor<'de> for BitVecVisitor {
            type Value = BitVec<Lsb0, u64>;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("Expected bitstring")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let mut vec = BitVec::<Lsb0, u64>::new();
                let s = String::from(value);
                for c in s.chars() {
                    let bit: bool = c.to_digit(2).unwrap() == 1;
                    vec.insert(0, bit)
                }
                Ok(vec)
            }
        }

        let val = deserializer.deserialize_str(BitVecVisitor)?;
        Ok(crate::values::Value { vec: val })
    }
}
