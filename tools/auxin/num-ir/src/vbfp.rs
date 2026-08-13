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
    // TODO: below could accept a closure for rounding / mapping behaviour in from and to directions
    // https://en.wikipedia.org/wiki/Fixed-point_arithmetic#Conversion_to_and_from_floating-point

    /// Create a fixed-point representation of the value ``inp`` using the current [FixedDef]. The new fixed-point value will attempt to approximate the value of ``inp``, with rounding on even ties.
    pub fn from_fp_rounded(
        &self,
        inp: f64,
    ) -> Result<BitVecValue, NumParseErr> {
        let scale: f64 = f64::powi(2., self.exp_mag);
        let scaled_inp = (inp * scale).round_ties_even();
        let inner_val = if self.signed {
            BitVecValue::from_i64(scaled_inp as i64, self.total_size as u32)
        } else if inp >= 0. {
            BitVecValue::from_u64(scaled_inp as u64, self.total_size as u32)
        } else {
            return Err(NumParseErr::Misc(format!(
                "can't read {inp} as unsigned fixed-point"
            )));
        };
        Ok(inner_val)
    }

    /// Given ``inp``, return the corresponding ``f64`` using the current [FixedDef].
    pub fn to_fp_rounded(&self, inp: &BitVecValue) -> f64 {
        let in_as_fp = if self.signed {
            inp.to_i64().map(|e| e as f64)
        } else {
            inp.to_u64().map(|e| e as f64)
        };
        let Some(v) = in_as_fp else {
            panic!("can't fit input to 64 bits")
        };
        v * (f64::powi(2., -self.exp_mag))
    }

    #[cfg(feature = "rand1")]
    /// generate a fixed-point value within ``bound``. if the [FixedDef] is signed, negative values will be generated; else positive only.
    ///
    /// bound must be >= 0.
    pub fn rand_fixed_bounded(
        &self,
        bound: f64,
        rng: &mut impl rand::Rng,
    ) -> BitVecValue {
        use rand::RngExt;

        assert!(bound >= 0.0);
        let rv: f64 = if self.signed {
            rng.random_range(-bound..bound)
        } else {
            rng.random_range(0.0..bound)
        };
        self.from_fp_rounded(rv).unwrap()
    }
}

/// given a fixed-point definition and expected/got values, determine whether they're really different, or if any inequality is simply due to fixed-points' limit on precision
pub fn within_precision(fd: &FixedDef, expc: f64, got: &BitVecValue) -> bool {
    // NOTE: vbfp is currently limited by fidelity of f64, so this relatively naive imeplementations should work. more precision will likely require implementation changes
    let got_fl = fd.to_fp_rounded(got);
    let diff = (got_fl - expc).abs();
    // TODO: would taking log2(diff) of both sides of this comparison be better?
    diff < 0.5 * f64::exp2(-fd.exp_mag as f64)
}

#[cfg(test)]

mod tests {
    use super::*;

    use rand::RngExt;
    #[test]
    fn within_precision_test() {
        let expc = 0.7;
        let fd = FixedDef {
            total_size: 32,
            exp_mag: 4,
            signed: false,
        };
        let t = fd.from_fp_rounded(expc).unwrap(); // should be 0.6875
        assert!(within_precision(&fd, expc, &t));
        assert!(!within_precision(&fd, 0.75, &t));

        // TODO: below should be more like a proptest
        for _ in 0..100 {
            let mut r = rand::rng();
            let v: f64 = r.random_range(0.0..0.03);
            let t = BitVecValue::zero(32);
            assert!(within_precision(&fd, v, &t))
        }
    }
}
