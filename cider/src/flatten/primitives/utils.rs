use crate::errors::{ConflictingAssignments, RuntimeError, RuntimeResult};
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
#[derive(Clone)]
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
    /// Shifts an option on to the front of the buffer and returns the option
    /// on the end of the buffer.
    pub fn shift(&mut self, element: Option<T>) -> Option<T> {
        let out = self.buffer.pop_back().flatten();
        self.buffer.push_front(element);
        out
    }

    /// Shifts an element out of the buffer without adding a new one
    pub fn shift_none(&mut self) -> Option<T> {
        self.shift(None)
    }

    /// Shifts an element on to the buffer and returns the option removed off
    /// the end of the buffer
    pub fn shift_new(&mut self, element: T) -> Option<T> {
        self.shift(Some(element))
    }

    /// Removes all instantiated elements in the buffer and replaces them with
    /// empty slots
    pub fn reset(&mut self) {
        for x in self.buffer.iter_mut() {
            *x = None
        }
    }

    pub fn all<F>(&self, query: F) -> bool
    where
        F: Fn(&Option<T>) -> bool,
    {
        self.buffer.iter().all(query)
    }
}

impl<T: PartialEq, const N: usize> ShiftBuffer<T, N> {
    /// Returns true if all entries in the shift buffer ar full and equal the
    /// given element
    pub fn all_equal_to(&self, query: &T) -> bool {
        self.buffer.iter().all(|x| x.as_ref() == Some(query))
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

use crate::flatten::{
    flat_ir::indexes::PortValue, structures::thread::ThreadIdx,
};

pub fn infer_thread_id<'a, I: Iterator<Item = &'a PortValue>>(
    iter: I,
) -> Option<ThreadIdx> {
    let mut result = None;
    for val in iter {
        // We have seen a thread id before
        if let Some(res) = result {
            if let Some(thread) = val.as_option().and_then(|x| x.thread()) {
                // If there are multiple thread ids, we return None
                if res != thread {
                    return None;
                }
            }
        }
        // Have not seen a thread id yet, can just take the possible empty id
        // from value
        else {
            result = val.as_option().and_then(|x| x.thread());
        }
    }
    result
}

#[derive(Clone)]
pub struct UnsynAssert {
    base_port: GlobalPortIdx,
    done_is_high: bool,
}

impl UnsynAssert {
    declare_ports![IN: 0, EN: 1, _CLK:2, RESET: 3, GO: 4, | OUT: 5, DONE: 6];
    pub fn new(base_port: GlobalPortIdx) -> Self {
        Self {
            base_port,
            done_is_high: false,
        }
    }

    fn exec_cycle(&mut self, port_map: &mut PortMap) -> RuntimeResult<()> {
        ports![&self.base_port;
            _in: Self::IN,
            en: Self::EN,
            reset: Self::RESET,
            go: Self::GO,
            out: Self::OUT,
            done: Self::DONE
        ];
        if port_map[en].as_bool().unwrap_or_default()
            && !port_map[_in].as_bool().unwrap_or_default()
        {
            RuntimeError::AssertionError();
        }
        Ok(());
    }
}
