use num_bigint::{BigInt, BigUint, Sign};
use std::collections::VecDeque;

/// There's probably a better way to do this but I'm not fixing it right now
pub(crate) fn floored_division(left: &BigInt, right: &BigInt) -> BigInt {
    let div = left / right;

    if left.sign() != Sign::Minus && right.sign() != Sign::Minus {
        div
    } else if (div.sign() == Sign::Minus || div.sign() == Sign::NoSign)
        && (left != &(&div * right))
    {
        div - 1_i32
    } else {
        div
    }
}

/// Implementation of integer square root via a basic binary search algorithm
/// based on wikipedia pseudocode.
///
/// TODO griffin: See if this can be replaced with a `sqrt` function in the
/// `num-bigint` crate
pub(crate) fn int_sqrt(i: &BigUint) -> BigUint {
    let mut lower: BigUint = BigUint::from(0_u32);
    let mut upper: BigUint = i + BigUint::from(1_u32);
    let mut temp: BigUint;

    while lower != (&upper - BigUint::from(1_u32)) {
        temp = (&lower + &upper) / BigUint::from(2_u32);
        if &(&temp * &temp) <= i {
            lower = temp
        } else {
            upper = temp
        }
    }
    lower
}

/// A shift buffer of a fixed size
pub struct ShiftBuffer<T, const N: usize> {
    buffer: VecDeque<Option<T>>,
}

impl<T, const N: usize> Default for ShiftBuffer<T, N> {
    fn default() -> Self {
        let mut buffer = VecDeque::with_capacity(N);
        for _ in 0..N {
            buffer.push_front(None)
        }
        Self { buffer }
    }
}

impl<T, const N: usize> ShiftBuffer<T, N> {
    /// Shifts an element on to the front of the buffer and returns the element
    /// on the end of the buffer.
    pub fn shift(&mut self, element: Option<T>) -> Option<T> {
        let out = self.buffer.pop_back().flatten();
        self.buffer.push_front(element);
        out
    }

    /// Removes all instantiated elements in the buffer and replaces them with
    /// empty slots
    pub fn reset(&mut self) {
        for x in self.buffer.iter_mut() {
            *x = None
        }
    }
}

/// An internal macro which is used to extract parameter values from an
/// association list input. Structured as a declaration list
macro_rules! get_params {
    ($inputs:ident; $( $param:ident : $id_name:expr ),+ ) => {
        $( let mut $param = None; )+
        for (id, v) in $inputs {
            match id.as_ref() {
                $($id_name => {$param = Some(v);}), +
                _ => {}
            }
        }
        $(let $param: u64 = *$param.expect(format!("Missing parameter: {}", $id_name).as_ref()); )+
    }
}

pub(crate) use get_params;
