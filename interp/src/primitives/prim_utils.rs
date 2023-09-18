use crate::values::Value;
use calyx_ir as ir;
use std::collections::VecDeque;

pub(super) fn get_param<S>(params: &ir::Binding, target: S) -> Option<u64>
where
    S: Into<ir::Id>,
{
    let target = target.into();
    params
        .iter()
        .find_map(|(id, x)| if *id == target { Some(*x) } else { None })
}

pub(super) fn get_input<'a, S>(
    inputs: &[(calyx_ir::Id, &'a Value)],
    target: S,
) -> Option<&'a Value>
where
    S: Into<calyx_ir::Id>,
{
    let target = target.into();
    inputs.iter().find(|(id, _)| *id == target).map(|(_, v)| *v)
}

pub(super) fn get_input_unwrap<'a, S>(
    inputs: &[(calyx_ir::Id, &'a Value)],
    target: S,
) -> &'a Value
where
    S: Into<calyx_ir::Id>,
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

    /// Removes all instantiated elements in the buffer and replaces them with
    /// empty slots
    pub fn reset(&mut self) {
        self.buffer.clear();
        for _ in 0..N {
            self.buffer.push_front(None)
        }
    }
}

macro_rules! get_inputs {
    ( $inputs:ident; $port:ident $([$ty:tt])? : $id_name:expr )  => {
        let $port = $inputs
                        .iter()
                        .find(|(id, _)| id == $id_name)
                        .map(|x| x.1);

        get_inputs!($port $(,$ty)? );
    };

    ( $inputs:ident; $( $port:ident $([$ty:tt])? : $id_name:expr ),+ )  => {
        $( let mut $port = None; )+
        for (id, v) in $inputs {
            match id.as_ref() {
                $($id_name => { $port =  Some(v); } ),+
                _ => {}
            }
        }
        $( get_inputs!($port $(,$ty)? ); )+

    };

    ($port:ident) => {
        let $port: &$crate::values::Value = $port.unwrap();
    };

    ($port:ident, bool) => {
        let $port: bool = $port.unwrap().as_bool();
    };

    ($port:ident, u64) => {
        let $port: u64 = $port.unwrap().as_u64();
    };

    ($port:ident, i64) => {
        let $port: i64 = $port.unwrap().as_i64();
    };
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

macro_rules! output {
    ( $(($id:expr, $val:expr) ),+ ) => {
        vec![
            $((calyx_ir::Id::from($id), $val)),+
        ]
    }
}

// export the macros for local use
pub(crate) use get_inputs;
pub(crate) use get_params;
pub(crate) use output;
