use std::ops::Not;
use std::{fmt::Write, ops::Index};

use bitvec::prelude::*;
use fraction::Fraction;
use ibig::{ibig, ops::UnsignedAbs, IBig, UBig};
use itertools::Itertools;
use serde::de::{self, Deserialize, Visitor};
use serde::Serialize;

pub type BitString = BitVec<usize, Lsb0>;

/// Retrieves the unsigned fixed point representation of `v`. This splits the representation into
///  integral and fractional bits. The width of the integral bits is described as:
/// `total width - fractional_width`.
fn get_unsigned_fixed_point(v: &Value, fractional_width: usize) -> Fraction {
    let integer_width: usize = v.width() as usize - fractional_width;

    // Calculate the integral part of the value. For each set bit at index `i`, add `2^i`.
    let whole: Fraction = v
        .vec
        .iter()
        .rev() // ...since the integer bits are most significant.
        .take(integer_width)
        .zip((0..integer_width).rev()) // Reverse indices as well.
        .fold(0u64, |acc, (bit, idx)| -> u64 {
            acc | ((*bit as u64) << idx)
        })
        .into();

    // Calculate the fractional part of the value. For each set bit at index `i`, add `2^-i`.
    // This begins at `1`, since the first fractional index has value `2^-1` = `1/2`.
    let fraction: Fraction =
        v.vec.iter().rev().skip(integer_width).enumerate().fold(
            Fraction::from(0u64),
            |acc, (idx, bit)| -> Fraction {
                let denom: u64 = (*bit as u64) << (idx + 1);
                // Avoid adding Infinity.
                if denom == 0u64 {
                    acc
                } else {
                    acc + Fraction::new(1u64, denom)
                }
            },
        );
    whole + fraction
}

// Lsb0 means [10010] gives 0 at index 0, 1 at index 1, 0 at index 2, etc
// from documentation, usize is the best data type to use in bitvec.
#[derive(Debug)]
pub struct ValueError {}

#[derive(Debug, Clone)]
pub enum InputNumber {
    // unsigned
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    U(UBig),
    // signed
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    I(IBig),
    // usize
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

impl From<UBig> for InputNumber {
    fn from(u: UBig) -> Self {
        Self::U(u)
    }
}

impl From<IBig> for InputNumber {
    fn from(i: IBig) -> Self {
        Self::I(i)
    }
}

impl InputNumber {
    fn is_negative(&self) -> bool {
        match self {
            InputNumber::I8(i) => *i < 0,
            InputNumber::I16(i) => *i < 0,
            InputNumber::I32(i) => *i < 0,
            InputNumber::I64(i) => *i < 0,
            InputNumber::I128(i) => *i < 0,
            InputNumber::I(i) => *i < 0.into(),
            _ => false,
        }
    }

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
            InputNumber::U(_) => unimplemented!(),
            InputNumber::I(_) => unimplemented!(),
        }
    }

    fn as_bit_vec(&self) -> BitString {
        match self {
            InputNumber::U8(i) => BitVec::from_element(*i as usize),
            InputNumber::U16(i) => BitVec::from_element(*i as usize),
            InputNumber::U32(i) => BitVec::from_element(*i as usize),
            InputNumber::U64(i) => BitVec::from_element(*i as usize),
            InputNumber::U128(i) => {
                let lower = (i & (u64::MAX as u128)) as usize;
                let upper = ((i >> 64) & u64::MAX as u128) as usize;
                BitVec::from_slice(&[lower, upper])
            }
            InputNumber::I8(i) => BitVec::from_element(*i as usize),
            InputNumber::I16(i) => BitVec::from_element(*i as usize),
            InputNumber::I32(i) => BitVec::from_element(*i as usize),
            InputNumber::I64(i) => BitVec::from_element(*i as usize),
            InputNumber::I128(i) => {
                let lower = (i & (u64::MAX as i128)) as usize;
                let upper = ((i >> 64) & u64::MAX as i128) as usize;
                BitVec::from_slice(&[lower, upper])
            }
            InputNumber::Usize(i) => BitVec::from_element(*i),
            InputNumber::U(u) => {
                let bytes_64: Vec<_> = u
                    .to_le_bytes()
                    .into_iter()
                    .chunks(8)
                    .into_iter()
                    .map(|x| {
                        let mut acc: usize = 0;
                        for (byte_number, u) in x.enumerate() {
                            acc |= (u as usize) << (byte_number * 8)
                        }
                        acc
                    })
                    .collect();

                BitString::from_slice(&bytes_64)
            }
            InputNumber::I(i) => {
                if i.signum() == ibig!(-1) {
                    let mut carry = true;
                    // manually do the twos complement conversion
                    let fun: Vec<_> = i
                        .unsigned_abs()
                        .to_le_bytes()
                        .into_iter()
                        .chunks(8)
                        .into_iter()
                        .map(|x| {
                            let mut acc: usize = 0;
                            for (byte_number, u) in x.enumerate() {
                                acc |= (u as usize) << (byte_number * 8)
                            }
                            acc = acc.not();

                            if carry {
                                let (new_acc, new_carry) =
                                    acc.overflowing_add(1);
                                carry = new_carry;
                                acc = new_acc;
                            }
                            acc
                        })
                        .collect();

                    let mut bv = BitVec::from_slice(&fun);

                    if carry {
                        bv.push(true)
                    }

                    bv.truncate(bv.last_one().unwrap() + 1);

                    bv
                } else {
                    let unsigned: InputNumber = i.unsigned_abs().into();
                    unsigned.as_bit_vec()
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
/// The type of all inputs and outputs to all components in Calyx.
/// Wraps a BitVector.
pub struct Value {
    // Lsb0 means the 0th index contains the LSB. This is useful because
    // a 7-bit bitvector and 17-bit bitvector representing the number 6 have
    // ones in the same index.
    vec: BitString,
}

impl From<BitString> for Value {
    fn from(bv: BitString) -> Self {
        Self { vec: bv }
    }
}

impl Value {
    pub fn unsigned_value_fits_in(vec: &BitString, width: usize) -> bool {
        vec.len() <= width // obviously fits then
            || vec
                .last_one() // returns an index
                .map(|x| x < width)
                .unwrap_or(true) // if there is no high bit then it can fit in the given width
    }

    pub fn signed_value_fits_in(vec: &BitString, width: usize) -> bool {
        vec.len() <= width // obviously fits then
        || (vec.ends_with(bits![0]) && Value::unsigned_value_fits_in(vec, width - 1)) // positive value (technically wastes a check)
        || (vec.ends_with(bits![1]) && ((vec.len() - vec.trailing_ones()) < width) || vec.trailing_ones() == 0)
        // negative value greater than or equal to lowest in new width
    }

    pub fn width(&self) -> u64 {
        self.vec.len() as u64
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = bool> + '_ {
        self.vec.iter().by_vals()
    }

    pub fn clone_bit_vec(&self) -> BitString {
        self.vec.clone()
    }

    pub fn bv_ref(&self) -> &BitString {
        &self.vec
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
            vec: bitvec![usize, Lsb0; 0; input_num.as_usize()],
        }
    }

    pub fn bit_high() -> Value {
        Value::from(1_usize, 1_usize)
    }

    pub fn bit_low() -> Value {
        Value::from(0_usize, 1_usize)
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
        let (v, _) = Value::from_checked(initial_val, bitwidth);
        v
    }

    /// Returns a bit vector for the given input value of the desired width and a bool
    /// representing whether the given value could fit in the required width. The result
    /// is truncated if it cannot fit.
    pub fn from_checked<T1: Into<InputNumber>, T2: Into<InputNumber>>(
        initial_val: T1,
        bitwidth: T2,
    ) -> (Self, bool) {
        let init: InputNumber = initial_val.into();
        let width: InputNumber = bitwidth.into();
        let width = width.as_usize();
        let mut bv = init.as_bit_vec();

        let flag = init.is_negative()
            && !Value::signed_value_fits_in(&bv, width)
            || !init.is_negative()
                && !Value::unsigned_value_fits_in(&bv, width);

        bv.resize(width, init.is_negative());
        (Value { vec: bv }, flag)
    }

    #[inline]
    pub fn from_bv(bv: BitString) -> Self {
        bv.into()
    }
    /// Returns a Value truncated to length new_size.
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

    pub fn truncate_in_place(&mut self, new_size: usize) {
        self.vec.truncate(new_size);
    }

    /// returns a value which consists of all the bits from left as the upper
    /// bits and all the bits from right as the lower bits of a value whose
    /// width is the sum of the widths of left and right
    pub fn concat(left: &Self, right: &Self) -> Self {
        Value {
            vec: left.vec.iter().chain(right.vec.iter()).collect(),
        }
    }

    /// Zero-extend the vector to length ext.
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

    /// Sign-extend the vector to length ext.
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

    /// Converts value into u64 type.
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let unsign_64_16 = Value::from(16, 16).as_u64();
    /// ```
    pub fn as_u64(&self) -> u64 {
        assert!(
            Value::unsigned_value_fits_in(&self.vec, 64),
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

    /// Converts value into unsigned fixed point type.
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// use fraction::Fraction;
    /// let ufixed_point_Q4_2 = Value::from(0b0110, 4).as_ufp(2); // 3/2
    /// ```
    pub fn as_ufp(&self, fractional_width: usize) -> Fraction {
        assert!(
            Value::unsigned_value_fits_in(&self.vec, 64),
            "unsigned fixed point is supported up to 64 bits. Open an issue if you require more."
        );
        get_unsigned_fixed_point(self, fractional_width)
    }

    /// Converts value into signed fixed point type.
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// use fraction::Fraction;
    /// let sfixed_point_Q4_2 = Value::from(0b1110, 4).as_sfp(2); // -3/2
    /// ```
    pub fn as_sfp(&self, fractional_width: usize) -> Fraction {
        assert!(
            Value::signed_value_fits_in(&self.vec, 64),
            "signed fixed point is supported up to 64 bits. Open an issue if you require more."
        );
        match self.vec.last_one() {
            Some(end) if (end + 1) == self.vec.len() => {
                let mut vec = self.clone_bit_vec();
                // Flip each bit until the first "one". This is
                // similar to flipping all bits and adding one.
                let begin = vec.first_one().unwrap();
                for mut bit in vec.iter_mut().rev().take(end - begin) {
                    *bit = !*bit
                }
                -get_unsigned_fixed_point(
                    &Value::from_bv(vec),
                    fractional_width,
                )
            }
            // Either there are no set bits (zero) or this number is non-negative.
            _ => get_unsigned_fixed_point(self, fractional_width),
        }
    }

    /// Converts value into u128 type.
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
            Value::unsigned_value_fits_in(&self.vec, 128),
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

    /// Converts value into i64 type using 2C representation. Sign extends lower values.
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let signed_neg_1_4 = Value::from(15, 4).as_i64();
    /// assert_eq!(signed_neg_1_4, -1);
    /// ```
    pub fn as_i64(&self) -> i64 {
        assert!(
            Value::signed_value_fits_in(&self.vec, 64),
            "Cannot fit value into an i64"
        );
        let init = if *self.vec.last().unwrap() { -1 } else { 0 };
        self.vec.iter().enumerate().take(64).fold(
            init,
            |acc, (idx, bit)| -> i64 {
                (acc & (!(1 << idx))) | ((*bit as i64) << idx)
            },
        )
    }

    /// Converts value into i128 type using 2C representation. Sign extends lower values.
    ///
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
            Value::signed_value_fits_in(&self.vec, 128),
            "Cannot fit value into an i128"
        );
        let init = if *self.vec.last().unwrap() { -1 } else { 0 };
        self.vec.iter().enumerate().take(128).fold(
            init,
            |acc, (idx, bit)| -> i128 {
                (acc & (!(1 << idx))) | ((*bit as i128) << idx)
            },
        )
    }

    pub fn as_usize(&self) -> usize {
        assert!(
            Value::unsigned_value_fits_in(&self.vec, usize::BITS as usize),
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

    pub fn as_signed(&self) -> IBig {
        let mut acc: IBig = 0.into();

        // skip the msb for the moment
        for bit in self.vec.iter().take(self.vec.len() - 1).rev() {
            acc <<= 1;
            let bit: IBig = (*bit).into();
            acc |= bit
        }

        if let Some(bit) = self.vec.last() {
            if *bit {
                let neg: IBig = (-1).into();
                let two: IBig = (2).into();

                acc += neg * two.pow(self.vec.len() - 1)
            }
        }

        acc
    }

    pub fn as_unsigned(&self) -> UBig {
        let mut acc: UBig = 0_u32.into();
        for bit in self.vec.iter().rev() {
            acc <<= 1;
            let bit: UBig = (*bit).into();
            acc |= bit;
        }

        acc
    }

    /// Interprets a 1bit value as a bool, will not panic for non-1-bit values
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

    /// Returns a value containing the sliced region \[lower,upper\], consumes the original
    pub fn slice_out(self, upper_idx: usize, lower_idx: usize) -> Self {
        assert!(upper_idx >= lower_idx);
        assert!(upper_idx < self.vec.len());

        let new_bv = (self.vec[lower_idx..=upper_idx]).into();
        Value { vec: new_bv }
    }

    /// Returns a value containing the sliced region \[lower,upper\]
    pub fn slice(&self, upper_idx: usize, lower_idx: usize) -> Self {
        assert!(upper_idx >= lower_idx);
        assert!(upper_idx < self.vec.len());

        let new_bv = BitVec::from_bitslice(&self.vec[lower_idx..=upper_idx]);
        Value { vec: new_bv }
    }

    /// Creates a value from a byte slice and truncates to the specified width.
    /// The bytes are assumed to be little-endian. The slice of bytes must be
    /// non-empty. And the width must be less than or equal to the number of
    /// bits in the slice. In cases where the width of the value is less than
    /// the bits provided, the unused upper values should be set to zero and will be
    /// discarded.
    pub fn from_bytes_le(bytes: &[u8], width: usize) -> Self {
        assert!(!bytes.is_empty());
        assert!(width <= bytes.len() * 8);
        // TODO griffin: Make this sanity check even mildly comprehensible
        let overhead = (width.div_ceil(8) * 8) - width;
        assert!(
            bytes.last().unwrap().leading_zeros() >= overhead as u32,
            "The upper byte of the provided value has non-zero values in the padding. Given byte is {} but the upper {} bit(s) should be zero",
            bytes.last().unwrap(),
            overhead
        );

        let chunks = bytes.chunks_exact(8);
        let remainder = chunks.remainder();

        let mut vec: Vec<usize> = chunks
            .map(|x| {
                usize::from_le_bytes([
                    x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7],
                ])
            })
            .collect();

        if !remainder.is_empty() {
            let mut acc = 0_usize;
            for (byte_number, u) in remainder.iter().enumerate() {
                acc |= (*u as usize) << (byte_number * 8)
            }
            vec.push(acc);
        }

        let mut bv = BitString::from_vec(vec);
        bv.truncate(width);
        Value { vec: bv }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // there has got to be a better way to do this
        self.vec
            .chunks(8)
            .map(|bits| {
                let mut byte = 0_u8;
                for (i, bit) in bits.iter().enumerate() {
                    byte |= (*bit as u8) << i;
                }
                byte
            })
            .collect()
    }
}

/* ============== Impls for Values to make them easier to use ============= */
#[allow(clippy::from_over_into)]
impl Into<u64> for Value {
    fn into(self) -> u64 {
        self.as_u64()
    }
}

impl Index<usize> for Value {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vec[index]
    }
}

impl std::fmt::Display for Value {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        let mut out = String::new();
        write!(out, "[")?;
        for bit in self.vec.iter().rev() {
            write!(out, "{}", i32::from(*bit))?;
        }
        write!(out, "]")?;
        write!(f, "{}", out)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.vec.len() == other.vec.len() && *self.vec == *other.vec
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
            .by_refs()
            .rev()
            .zip(other.vec.iter().by_refs().rev())
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

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        base64::encode(self.as_unsigned().to_le_bytes()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BitVecVisitor;

        impl<'de> Visitor<'de> for BitVecVisitor {
            type Value = (UBig, usize);

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
                let s = base64::decode(value)
                    .expect("Couldn't convert from base64");
                Ok((UBig::from_le_bytes(&s), s.len()))
            }
        }

        let (val, bytes) = deserializer.deserialize_str(BitVecVisitor)?;
        Ok(Value::from(val, bytes * 8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_byte_roundtrip(data in proptest::collection::vec(any::<u8>(), 1..100)) {
            // this doesn't really test the truncation since it's been hard to
            // get that working in a way that still generates values correctly
            // but this is good enough for now
            let val = Value::from_bytes_le(&data, data.len() * 8);
            let bytes = val.to_bytes();
            prop_assert_eq!(bytes, data);
        }
    }
}
