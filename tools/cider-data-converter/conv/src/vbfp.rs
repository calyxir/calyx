// very bad fixed-point

use crate::numrep::*;

pub struct FixedDef {
    pub total_size: usize,
    pub exp_mag: i32,
}

impl FixedDef {
    // TODO: below could accept a closure for behaviour in either direction
    // https://en.wikipedia.org/wiki/Fixed-point_arithmetic#Conversion_to_and_from_floating-point

    pub fn from_fp_rounded(
        &self,
        inp: f64,
        signed: bool,
    ) -> Result<BinRep, ReadStringErr> {
        let scale: f64 = f64::powi(2., self.exp_mag);
        let scaled_inp = inp * scale;
        if signed {
            let corr_int = scaled_inp.round_ties_even() as i64;
            Ok(corr_int as u64)
        } else {
            Ok(scaled_inp.round_ties_even() as u64)
        }
    }

    // TODO: make a generic 'to_fp_closure'
    pub fn to_fp_rounded(&self, inp: &BinRep, signed: bool) -> f64 {
        if signed {
            let in_raw = *inp as u64;
            let msb_mask = 1 << (self.total_size - 1);
            let sgn_mask = !crate::util::mask_n_bits(self.total_size)
                & crate::util::mask_n_bits(64); // maximum width
            let extended = if in_raw & msb_mask != 0 {
                sgn_mask | in_raw
            } else {
                in_raw
            };
            let in_signed = extended as i64;

            let in_fp = in_signed as f64;
            in_fp * (f64::powi(2., -self.exp_mag))
        } else {
            let in_fp = *inp as f64;
            in_fp * (f64::powi(2., -self.exp_mag))
        }
    }
}
