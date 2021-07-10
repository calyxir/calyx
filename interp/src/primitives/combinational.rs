use super::{stateful::get_param, Primitive};
use crate::comb_primitive;
use crate::values::Value;
use calyx::ir;
use std::ops::Not;

/// A constant.
#[derive(Default, Debug)]
pub struct StdConst {
    value: Value,
}

impl StdConst {
    pub fn from_constants(value: u64, width: u64) -> Self {
        StdConst {
            value: Value::try_from_init(value, width).unwrap(),
        }
    }

    pub fn new(params: ir::Binding) -> Self {
        let width = get_param(&params, "WIDTH")
            .expect("Missing width parameter from std_const binding");

        let init_value = get_param(&params, "VALUE")
            .expect("Missing `vale` param from std_const binding");

        let value = Value::try_from_init(init_value, width).unwrap();

        Self { value }
    }
}

impl Primitive for StdConst {
    fn is_comb(&self) -> bool {
        true
    }

    fn validate(&self, _inputs: &[(ir::Id, &Value)]) {}

    fn execute(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
        _done_val: Option<&Value>,
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        vec![("out".into(), self.value.clone().into())]
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        vec![("out".into(), self.value.clone().into())]
    }

    fn commit_updates(&mut self) {}

    fn clear_update_buffer(&mut self) {}
}

// ===================== Unary operations ======================
comb_primitive!(StdNot[WIDTH](r#in: WIDTH) -> (out: WIDTH) {
    Value {
        vec: r#in.vec.clone().not(),
    }
    .into()
});

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

// ===================== Shift Operations ======================
comb_primitive!(StdLsh[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let mut tr = left.vec.clone();
    tr.shift_right(right.as_u64() as usize);
    Value { vec: tr }.into()
});
comb_primitive!(StdRsh[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let mut tr = left.vec.clone();
    tr.shift_left(right.as_u64() as usize);
    Value { vec: tr }.into()
});

// ===================== Logial Operations ======================
comb_primitive!(StdAnd[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Value {
        vec: left.vec.clone() & right.vec.clone(),
    }.into()
});
comb_primitive!(StdOr[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Value {
        vec: left.vec.clone() | right.vec.clone(),
    }.into()
});
comb_primitive!(StdXor[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Value {
        vec: left.vec.clone() ^ right.vec.clone(),
    }.into()
});

// ===================== Comparison Operations ======================
comb_primitive!(StdGt[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
  let left_64 = left.as_u64();
  let right_64 = right.as_u64();
  let init_val = left_64 > right_64;
  Value::from_init(init_val, 1_usize).into()
});
comb_primitive!(StdLt[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
  let left_64 = left.as_u64();
  let right_64 = right.as_u64();
  let init_val = left_64 < right_64;
  Value::from_init(init_val, 1_usize).into()
});
comb_primitive!(StdGe[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
  let left_64 = left.as_u64();
  let right_64 = right.as_u64();
  let init_val = left_64 >= right_64;
  Value::from_init(init_val, 1_usize).into()
});
comb_primitive!(StdLe[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
  let left_64 = left.as_u64();
  let right_64 = right.as_u64();
  let init_val = left_64 <= right_64;
  Value::from_init(init_val, 1_usize).into()
});
comb_primitive!(StdEq[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
  let left_64 = left.as_u64();
  let right_64 = right.as_u64();
  let init_val = left_64 == right_64;
  Value::from_init(init_val, 1_usize).into()
});
comb_primitive!(StdNeq[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
  let left_64 = left.as_u64();
  let right_64 = right.as_u64();
  let init_val = left_64 != right_64;
  Value::from_init(init_val, 1_usize).into()
});

// ===================== Resizing Operations ======================
comb_primitive!(StdSlice[IN_WIDTH, OUT_WIDTH](r#in: IN_WIDTH) -> (out: OUT_WIDTH) {
    let tr = r#in.clone();
    tr.truncate(OUT_WIDTH as usize).into()
});
comb_primitive!(StdPad[IN_WIDTH, OUT_WIDTH](r#in: IN_WIDTH) -> (out: OUT_WIDTH) {
    let pd = r#in.clone();
    pd.ext(OUT_WIDTH as usize).into()
});
