use std::collections::VecDeque;

use ibig::{ibig, UBig};
use ibig::{ubig, IBig};

pub(crate) fn floored_division(left: &IBig, right: &IBig) -> IBig {
    let div = left / right;

    if left.signum() != ibig!(-1) && right.signum() != ibig!(-1) {
        div
    } else if (div.signum() == (-1).into() || div.signum() == 0.into())
        && (left != &(&div * right))
    {
        div - 1_i32
    } else {
        div
    }
}

/// Implementation of integer square root via a basic binary search algorithm
/// based on wikipedia psuedocode
pub(crate) fn int_sqrt(i: &UBig) -> UBig {
    let mut lower: UBig = ubig!(0);
    let mut upper: UBig = i + ubig!(1);
    let mut temp: UBig;

    while lower != (&upper - ubig!(1)) {
        temp = (&lower + &upper) / ubig!(2);
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
