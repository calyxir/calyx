use crate::values::Value;
use calyx::ir;
use std::collections::VecDeque;

pub(super) fn get_param<S>(params: &ir::Binding, target: S) -> Option<u64>
where
    S: AsRef<str>,
{
    params.iter().find_map(|(id, x)| {
        if id == target.as_ref() {
            Some(*x)
        } else {
            None
        }
    })
}

pub(super) fn get_input<'a, S>(
    inputs: &[(calyx::ir::Id, &'a Value)],
    target: S,
) -> Option<&'a Value>
where
    S: AsRef<str>,
{
    inputs
        .iter()
        .find(|(id, _)| id == target.as_ref())
        .map(|(_, v)| *v)
}

pub(super) fn get_input_unwrap<'a, S>(
    inputs: &[(calyx::ir::Id, &'a Value)],
    target: S,
) -> &'a Value
where
    S: AsRef<str>,
{
    get_input(inputs, target).unwrap()
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
        self.buffer.push_front(element);
        // this is safe as the buffer will always have N + 1 elements before
        // this call
        self.buffer.pop_back().unwrap()
    }
}
