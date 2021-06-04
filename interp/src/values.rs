use bitvec::prelude::*;
use std::convert::TryInto;

// Lsb0 means [10010] gives 0 at index 0, 1 at index 1, 0 at index 2, etc
//from documentation, usize is the best data type to use in bitvec

#[derive(Clone, Debug)]
pub struct Value {
    pub vec: BitVec<Lsb0, usize>,
}

impl Value {
    fn new(bitwidth: usize) -> Value {
        Value {
            vec: BitVec::with_capacity(bitwidth),
        }
    }

    pub fn from_init<T: Into<usize>>(initial_val: T, bitwidth: usize) -> Self {
        let mut vec = BitVec::from_element(initial_val.into());
        vec.resize(bitwidth, false);
        Value { vec }
    }

    /// truncate returns a clone of [self] with [vec] of length [new_size]
    pub fn truncate(&self, new_size: usize) -> Value {
        //our methods are functional, so return a new value
        let mut vec = self.vec.clone();
        //now just truncate the vector in tr
        vec.truncate(new_size);
        Value {
            // vec: tr.vec.Bitvect::truncate(new_size)
            vec,
        }
    }

    /// [ext] returns a copy of [self] of length [ext], with the
    /// difference between [self.len] and [ext] made up by [0s]
    pub fn ext(&self, ext: usize) -> Value {
        let mut vec = self.vec.clone();
        for x in 0..(ext - vec.len()) {
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
        for x in 0..(ext - vec.len()) {
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
