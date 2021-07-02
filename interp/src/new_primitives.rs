use super::values::{OutputValue, Value};
use crate::comb_primitive;
use calyx::ir;

pub trait Primitive {
    /// Returns true if this primitive is combinational
    fn is_comb(&self) -> bool;

    /// Validate inputs to the component.
    fn validate(&self, inputs: &[(ir::Id, &Value)]);

    /// Execute the component.
    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        done_val: Option<&Value>,
    ) -> Vec<(ir::Id, OutputValue)>;

    /// Reset the component.
    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)>;

    /// Transfers the update held in a primitive's buffer into the
    /// state contained within the primitive itself. Until this method is
    /// invoked, the primitive's internal state will remain unchanged by
    /// execution. This is to prevent ephemeral changes due to repeated
    /// invocations
    fn commit_updates(&mut self);

    /// Resets the primitive's update buffer without commiting the held changes,
    /// effectively reverting the write and ensuring it does not occur. Use to
    /// reset stateful primitives after a group execution.
    fn clear_update_buffer(&mut self);
}

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
