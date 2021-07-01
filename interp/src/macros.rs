/// Define a primitive
///
/// ```
/// binary_primitive!(StdAdd[width](left: width, right: width) : {
///   let left_64 = left.as_u64();
///   let right_64 = right.as_u64();
///   let init_val = left_64 + right_64;
///   let bitwidth: usize = left.vec.len();
///   Value::from_init(init_val, bitwidth).into()
/// });
/// ```
#[macro_export]
macro_rules! primitive {
    ($name:ident[ $( $param:ident ),+]( $( $port:ident : $width:ident ),+ ) : $execute:block ) => {
        pub struct $name {
            $($param: u64),+
        }

        impl $name {
            pub fn new( $($param: u64),+ ) -> $name {
                $name { $($param),+ }
            }

            fn execute($( $port: &crate::values::Value),* ) -> crate::values::OutputValue {
                $execute
            }
        }

        impl crate::primitives::ValidateInput for $name {
            fn validate_input(
                &self,
                inputs: &[(calyx::ir::Id, &crate::values::Value)]) {
                for (id, v) in inputs {
                    match id.as_ref() {
                        $( stringify!($port) => assert_eq!(v.len() as u64, self.$width) ),+,
                        p => unreachable!(format!("Unknown port: {}", p)),
                    }
                }
            }
        }
    };
}
