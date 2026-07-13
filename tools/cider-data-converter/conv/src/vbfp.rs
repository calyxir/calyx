// very bad fixed-point

use baa::{BitVecOps, BitVecValue};

use crate::typing::ReadStringErr;

/// only supports sign via two's complement
/// signed-magnitude fixed-point is not supported.
pub struct FixedDef {
    pub total_size: usize,
    pub exp_mag: i32,
    pub signed: bool,
}

impl FixedDef {
    // TODO: below could accept a closure for behaviour in either direction
    // https://en.wikipedia.org/wiki/Fixed-point_arithmetic#Conversion_to_and_from_floating-point

    pub fn from_fp_rounded(
        &self,
        inp: f64,
    ) -> Result<BitVecValue, ReadStringErr> {
        let scale: f64 = f64::powi(2., self.exp_mag);
        let scaled_inp = inp * scale;
        if self.signed {
            let corr_int = scaled_inp.round_ties_even() as i64;
            Ok(BitVecValue::from_i64(corr_int, self.total_size as u32))
        } else {
            Ok(BitVecValue::from_u64(
                scaled_inp.round_ties_even() as u64,
                self.total_size as u32,
            ))
        }
    }

    // TODO: make a generic 'to_fp_closure'
    pub fn to_fp_rounded(&self, inp: &BitVecValue) -> f64 {
        let in_as_fp = if self.signed {
            let in_num = inp.to_i64().unwrap();

            in_num as f64
        } else {
            let in_num = inp.to_u64().unwrap();
            in_num as f64
        };
        in_as_fp * (f64::powi(2., -self.exp_mag))
    }
}
