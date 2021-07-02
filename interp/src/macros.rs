use crate::values::{OutputValue, Value};
use calyx::ir;

/// Define a primitive
///
/// ```
/// primitive!(StdAdd[width](left: width, right: width) {
///   let left_64 = left.as_u64();
///   let right_64 = right.as_u64();
///   let init_val = left_64 + right_64;
///   let bitwidth: usize = left.vec.len();
///   Value::from_init(init_val, bitwidth).into()
/// });
/// ```
/// The macro defines several methods for the struct `StdAdd`:
/// - StdAdd::new(width: u64): Construct a new StdAdd instance with `width`.
/// - StdAdd::execute(left: &Value, right: &Value): Runs the provided code block.
/// - StdAdd::validate_input(inputs: &[ir::Id, &values::Value]): Validate the
///   input ports have the appropriate widths.
///
/// TODO(rachit): $out_width is never used.
#[macro_export]
macro_rules! primitive {
    ($name:ident[ $( $param:ident ),+]( $( $port:ident : $width:ident ),+ ) -> ( $( $out:ident : $out_width:ident ),+ ) $execute:block ) => {
        #[derive(Clone, Debug, Default)]
        pub struct $name {
            $($param: u64),+
        }


        impl Executable for $name {

            fn new(params: ir::Binding) -> Self {
                let mut base = Self::default();
                for (param, value) in params {
                    match param.as_ref() {
                        $( stringify!($param) => base.$param = value ),+,
                        p => unreachable!(format!("Unknown parameter: {}", p)),
                    }
                }
                base
            }

            fn validate(
                &self,
                inputs: &[(ir::Id, &Value)]
            ) {
                for (id, v) in inputs {
                    match id.as_ref() {
                        $( stringify!($port) => assert_eq!(v.len() as u64, self.$width) ),+,
                        p => unreachable!(format!("Unknown port: {}", p)),
                    }
                }
            }

            fn execute(
                &mut self,
                inputs: &[(ir::Id, &Value)],
                // done_val not used in combinational primitives
                _done_val: Option<&Value>) -> Vec<(ir::Id, OutputValue)> {

                #[derive(Default)]
                struct Ports<'a> {
                    $( $port: Option<&'a Value> ),+
                }

                let mut base = Ports::default();

                for (id, v) in inputs {
                    match id.as_ref() {
                        $( stringify!($port) => base.$port = Some(v) ),+,
                        p => unreachable!(format!("Unknown port: {}", p)),
                    }
                }

                let exec_func = |$( $port: &Value ),+| -> OutputValue {
                    $execute
                };

                let $( $out ),+ = exec_func(
                    // TODO(rachit): Better error if this fails
                    $( base.$port.unwrap() ),+
                );

                return vec![
                    $( (stringify!($out).into(), $out) ),+
                ]

            }

            fn commit_updates(&mut self) {
                todo!()
            }

            fn clear_update_buffer(&mut self) {
                todo!()
            }
        }
    };
}
