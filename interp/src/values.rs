use bitvec::prelude::*;
use serde::de::{self, Deserialize, Visitor};
use std::convert::TryInto;

// Lsb0 means [10010] gives 0 at index 0, 1 at index 1, 0 at index 2, etc
// from documentation, usize is the best data type to use in bitvec.
#[derive(Debug)]
pub struct ValueError {}

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
    pub fn zeroes(bitwidth: usize) -> Value {
        Value {
            vec: bitvec![Lsb0, u64; 0; bitwidth],
        }
    }

    pub fn bit_high() -> Value {
        Value::from_init(1_u64, 1_usize)
    }

    pub fn bit_low() -> Value {
        Value::from_init(0_u64, 1_usize)
    }

    /// Creates a new Value of a given bitwidth out of an initial_val. It's
    /// safer to use [from] followed by [unwrap].
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let val_16_16 = Value::from_init(16 as u64, 16 as usize);
    /// ```
    pub fn from_init<T1: Into<u64>, T2: Into<usize>>(
        initial_val: T1,
        bitwidth: T2,
    ) -> Self {
        let mut vec = BitVec::from_element(initial_val.into());
        vec.resize(bitwidth.into(), false);
        Value { vec }
    }

    /// Create a new Value of a given bitwidth out of an initial_val. You do
    /// not have to guarantee initial_val satisifies Into<u64>, or bitwidth
    /// satisfies Into<usize>.
    ///
    /// # Example:
    /// ```
    /// use interp::values::*;
    /// let val_16_16 = Value::from(16, 16).unwrap();
    /// ```
    pub fn from<T1, T2>(
        initial_val: T1,
        bitwidth: T2,
    ) -> Result<Self, ValueError>
    where
        T1: TryInto<u64>,
        T2: TryInto<usize>,
    {
        let (val, width): (u64, usize) =
            match (initial_val.try_into(), bitwidth.try_into()) {
                (Ok(v1), Ok(v2)) => (v1, v2),
                _ => return Err(ValueError {}),
            };

        let mut vec = BitVec::from_element(val);
        vec.resize(width, false);
        Ok(Value { vec })
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
    /// let val_4_4 = (Value::from(4, 16).unwrap()).truncate(4);
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
    /// let val_4_16 = (Value::from(4, 4).unwrap()).ext(16);
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
    /// let val_31_5 = (Value::from(15, 4).unwrap()).sext(5);
    /// ```
    pub fn sext(&self, ext: usize) -> Value {
        let mut vec = self.vec.clone();
        let sign = vec[vec.len() - 1];
        for _x in 0..(ext - vec.len()) {
            vec.push(sign);
        }
        Value { vec }
    }

    /// Converts value into u64 type. Vector within Value can be of any width.
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let unsign_64_16 = (Value::from(16, 16).unwrap()).as_u64();
    /// ```
    pub fn as_u64(&self) -> u64 {
        let mut val: u64 = 0;
        for (index, bit) in self.vec.iter().by_ref().enumerate() {
            if *bit {
                //protects against panic in case of # less than u64::max in
                // value of width greater than 64
                val |= 1 << index;
            }
        }
        val
    }

    /// Converts value into u128 type. Vector within Value can be of any width.
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let unsign_128_16 = (Value::try_from_init(16, 16).unwrap()).as_u128();
    /// ```
    pub fn as_u128(&self) -> u128 {
        let mut val: u128 = 0;
        for (index, bit) in self.vec.iter().by_ref().enumerate() {
            if *bit {
                val |= 1 << index;
            }
        }
        val
    }

    /// Converts value into i64 type using 2C representation.
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let signed_neg_1_4 = (Value::try_from_init(15, 4).unwrap()).as_i64();
    /// assert_eq!(signed_neg_1_4, -1);
    /// ```
    pub fn as_i64(&self) -> i64 {
        let vec_len = self.vec.len() as u32;
        if vec_len == 0 {
            return 0;
        }
        let pow_base = -2;
        let msb_weight = i64::pow(pow_base, vec_len - 1);
        let mut tr: i64 = 0;
        let iter = self.vec.iter().by_ref();
        //which way will it iterate? Hopefully w/ LsB = 0
        for (place, b) in iter.enumerate() {
            if *b {
                if place >= (vec_len - 1) as usize {
                    //2s complement, so MSB has negative weight
                    //this is the last place
                    tr += msb_weight;
                } else {
                    //before MSB, increase as unsigned bitnum
                    tr += i64::pow(2, place as u32); //
                }
            }
        }
        tr
    }

    #[allow(clippy::len_without_is_empty)]
    /// Returns the length (bitwidth) of the value
    ///
    /// # Example
    /// ```
    /// use interp::values::*;
    /// let v = Value::from_init(1_u16, 3_u16);
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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        assert!(self.vec.len() == other.vec.len());
        Some(self.as_u64().cmp(&other.as_u64()))
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
