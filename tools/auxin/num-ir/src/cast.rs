use crate::numrep::*;

// currently not included: types to facilitate easier implementation of casts

pub enum OpTypes {
    Truncate,
    Bitcast,
    SignExtend,
    Cast(OpCastTypes),
}

#[derive(PartialEq, Eq, Hash)]
pub enum OpCastTypes {
    SatCast, // saturating cast: i.e. if over/underflow, retains max or min value. rounding.
    WrapCast, // if over/underflows, treats 'remaining' part as valid. rounding.
    OverflowCast, // falliable cast which errors on overflow. rounding.
    LosslessCast, // falliable cast which attempts to preserve interpreted '=value' (i.e. -1.0_f32 -> -1_i32), and errors if not possible to represent value.
}

#[derive(Default)]
pub struct CastMap {
    // mapping of (in_t, out_t, cast_t) to a function
}

impl CastMap {
    pub fn get_cast(
        &self,
        itype: &TypeSpec,
        otype: &TypeSpec,
        op: OpCastTypes,
    ) -> Result<OpFnTypes, OpError> {
        unimplemented!();
    }
}

/*

NOTE: casts are completely unimplemented
*/
