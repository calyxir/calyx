use super::Primitive;
use crate::comb_primitive;
use crate::values::Value;

// ===================== Unsigned binary operations ======================
comb_primitive!(StdAdd[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
  let left_64 = left.as_u64();
  let right_64 = right.as_u64();
  let init_val = left_64 + right_64;
  let bitwidth: usize = left.vec.len();
  Value::from_init(init_val, bitwidth).into()
});
comb_primitive!(StdSub[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
  let left_64 = left.as_u64();
  let right_64 = right.as_u64();
  let init_val = left_64 - right_64;
  let bitwidth: usize = left.vec.len();
  Value::from_init(init_val, bitwidth).into()
});
