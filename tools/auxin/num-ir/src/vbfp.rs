// very bad fixed-point

use baa::{BitVecOps, BitVecValue};

use crate::typing::NumParseErr;

/// only supports sign via two's complement.
///
/// signed-magnitude fixed-point is not supported.
pub struct FixedDef {
    pub total_size: usize,
    pub exp_mag: i32,
    pub signed: bool,
}

impl FixedDef {
    // TODO: below could accept a closure for behaviour in either direction
    // https://en.wikipedia.org/wiki/Fixed-point_arithmetic#Conversion_to_and_from_floating-point

    /// Create a fixed-point representation of the value ``inp`` using the current [FixedDef]. The new fixed-point value will attempt to approximate the value of ``inp``, with rounding on even ties.
    pub fn from_fp_rounded(
        &self,
        inp: f64,
    ) -> Result<BitVecValue, NumParseErr> {
        let scale: f64 = f64::powi(2., self.exp_mag);
        let scaled_inp = inp * scale;
        if self.signed {
            let corr_int = scaled_inp.round_ties_even() as i64;
            Ok(BitVecValue::from_i64(corr_int, self.total_size as u32))
        } else {
            if inp < 0. {
                return Err(NumParseErr::Misc(format!(
                    "can't read {inp} as unsigned fixed-point"
                )));
            }
            Ok(BitVecValue::from_u64(
                scaled_inp.round_ties_even() as u64,
                self.total_size as u32,
            ))
        }
    }

    // TODO: make a generic 'to_fp_closure'
    /// Given ``inp``, return the corresponding ``f64`` using the current [FixedDef].
    pub fn to_fp_rounded(&self, inp: &BitVecValue) -> f64 {
        let in_as_fp = if self.signed {
            let Some(in_num) = inp.to_i64() else {
                panic!(
                    "input cannot be put into 64 bits, thus cannot fit in f64"
                )
            };

            in_num as f64
        } else {
            let Some(in_num) = inp.to_u64() else {
                panic!(
                    "input cannot be put into 64 bits, thus cannot fit in f64"
                )
            };
            in_num as f64
        };
        in_as_fp * (f64::powi(2., -self.exp_mag))
    }

    #[cfg(feature = "rand1")]
    pub fn rand_fixed_bounded(
        &self,
        int_bits: usize,
        exp_bits: usize,
    ) -> BitVecValue {
        // generate a fixed-point with at most ``int_bits`` of integer magnitude, ``exp_bits`` of exponent magnitude
        // exp_bits will be disregarded if self.exp_mag < 0
        unimplemented!()
    }
    // TODO: a function for generating bounds?
}

/// given a fixed-point definition and expected/got values, determine whether they're really different, or if any inequality is simply due to fixed-points' limit on precision
pub fn within_precision(fd: &FixedDef, expc: f64, got: &BitVecValue) -> bool {
    unimplemented!()
}

#[cfg(feature = "rand1")]
pub fn rand_fixed_def() -> FixedDef {
    // generate a random fixed-point definition, with only positive exp_mag produced.
    todo!()
}
