use bitvec::prelude::*;
use std::convert::TryInto;

// Lsb0 means [10010] gives 0 at index 0, 1 at index 1, 0 at index 2, etc
//from documentation, usize is the best data type to use in bitvec
#[derive(Debug)]
pub struct ValueError {}

#[derive(Clone, Debug)]
pub struct Value {
    pub vec: BitVec<Lsb0, u64>,
}

impl Value {
    pub fn new(bitwidth: usize) -> Value {
        Value {
            vec: BitVec::with_capacity(bitwidth),
        }
    }

    pub fn zeroes(bitwidth: usize) -> Value {
        Value {
            vec: bitvec![Lsb0, u64; 0; bitwidth],
        }
    }

    pub fn from_init<T1: Into<u64>, T2: Into<usize>>(
        initial_val: T1,
        bitwidth: T2,
    ) -> Self {
        let mut vec = BitVec::from_element(initial_val.into());
        vec.resize(bitwidth.into(), false);
        Value { vec }
    }

    pub fn try_from_init<T1, T2>(
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

    ///Note that Value is a functional data structure. This returns
    ///A Value with uninitialized data.
    pub fn clear(&self) -> Self {
        let mut vec = self.vec.clone();
        vec.truncate(0);
        Value { vec }
    }

    /// truncate returns a clone of [self] with [vec] of length [new_size]
    pub fn truncate(&self, new_size: usize) -> Value {
        //our methods are functional, so return a new value
        let mut vec = self.vec.clone();
        //now just truncate the vector in tr
        vec.truncate(new_size);
        Value { vec }
    }

    /// [ext] returns a copy of [self] of length [ext], with the
    /// difference between [self.len] and [ext] made up by [0s]
    pub fn ext(&self, ext: usize) -> Value {
        let mut vec = self.vec.clone();
        for _x in 0..(ext - vec.len()) {
            vec.push(false);
        }
        Value { vec }
    }

    /// sign-extend returns a copy of [self] of length [ext], with
    /// the difference between [self.len] and [ext] made up by t
    /// 1 if [self] is negative and 0 if [self] is positive
    pub fn sext(&self, ext: usize) -> Value {
        let mut vec = self.vec.clone();
        let sign = vec[vec.len() - 1];
        for _x in 0..(ext - vec.len()) {
            vec.push(sign);
        }
        Value { vec }
    }

    pub fn as_u64(&self) -> u64 {
        let mut val: u64 = 0;
        for (index, bit) in self.vec.iter().by_ref().enumerate() {
            val += u64::pow(2, (index as usize).try_into().unwrap())
                * (*bit as u64);
        }
        val
    }
}

impl Into<u64> for Value {
    fn into(self) -> u64 {
        let mut val: u64 = 0;
        for (index, bit) in self.vec.into_iter().enumerate() {
            val += u64::pow(2, (index as usize).try_into().unwrap())
                * (bit as u64);
        }
        val
    }
}

// For testing
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
