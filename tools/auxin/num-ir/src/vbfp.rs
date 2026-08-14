//! very bad fixed-point, a simple runtime fixed-point library

use baa::{BitVecOps, BitVecValue};

use crate::typing::NumParseErr;

/// a definition for a fixed-point type.
///
/// only supports sign via two's complement.
///
/// signed-magnitude fixed-point is not supported.
pub struct FixedDef {
    /// Width of whole type
    pub total_size: usize,
    /// Magnitude of exponent. The 'true' value which a set of bits represents is
    /// ``int(bits) * 2**(-exp_mag)``
    pub exp_mag: i32,
    pub signed: bool,
}

impl FixedDef {
    pub fn max(&self) -> f64 {
        let int_len = (self.total_size - self.exp_mag as usize) as i32;
        if self.signed {
            f64::powi(2., int_len - 1) - 1.
        } else {
            f64::powi(2., int_len) - 1.
        }
    }

    pub fn min(&self) -> f64 {
        if !self.signed {
            0.
        } else {
            let int_len = (self.total_size - self.exp_mag as usize) as i32;

            -1. * f64::powi(2., int_len - 1)
        }
    }
    // TODO: below could accept a closure for rounding / mapping behaviour in from and to directions
    // https://en.wikipedia.org/wiki/Fixed-point_arithmetic#Conversion_to_and_from_floating-point

    /// Create a fixed-point representation of the value ``inp`` using the current [FixedDef]. The new fixed-point value will attempt to approximate the value of ``inp``, with rounding on even ties.
    ///
    /// Out of bounds behaviour: if the input is too large to be represented within the integer bits of the fixed-point, the closest possible value will be returned.
    pub fn from_fp_rounded(
        &self,
        inp: f64,
    ) -> Result<BitVecValue, NumParseErr> {
        let clamped_in = inp.clamp(self.min(), self.max());
        let scale: f64 = f64::powi(2., self.exp_mag);
        let scaled_inp = (clamped_in * scale).round_ties_even(); // f64, should be representable with integer

        // https://doc.rust-lang.org/reference/expressions/operator-expr.html#type-cast-expressions
        // following doesn't use to_bits as we want the actual integer value

        let inner_val = if self.signed {
            let corr_s = (scaled_inp) as i64;
            BitVecValue::from_i64(corr_s, 64)
        } else if inp >= 0. {
            let corr_us = scaled_inp as u64;
            BitVecValue::from_u64(corr_us, 64)
        } else {
            return Err(NumParseErr::Misc(format!(
                "can't read {inp} as unsigned fixed-point"
            )));
        };

        Ok(inner_val.slice((self.total_size as u32) - 1, 0))
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
    // the exact 0.5 bound is to handle cases where last bit got rounded up
    diff <= 0.5 * f64::exp2(-fd.exp_mag as f64)
}

#[cfg(test)]

mod tests {
    use super::*;

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
    }

    use proptest::prelude::*;
    proptest! {
        #[test]
        fn within_precision_prop(f in -1e4f32..1e4, exp in 5..50){
            // NOTE: increasing the range of ``f`` too much can lead to overflows of 64-bits and/or values which are too large to fit in the exponent.
            // if increasing bounds on f, set highest exp to (64 - log2(max(f)))
            let fd = FixedDef{
                total_size: 64,
                exp_mag: exp,
                signed: (f < 0.)
            };
            let corr_fixed = fd.from_fp_rounded(f.into()).unwrap();
            prop_assert!(within_precision(&fd, f.into(), &corr_fixed))
        }

        #[test]
        fn overflow_unsigned(f in 17f32..255.){
            let fd = FixedDef{
                total_size: 8,
                exp_mag: 4,
                signed: false
            };
            let corr_fixed = fd.from_fp_rounded(f.into()).unwrap();
            prop_assert_eq!(corr_fixed.to_u64().unwrap() >> 4, 0xf);

        }

        #[test]
        fn overflow_signed_hi(f in 9f32..255.){
            let fd = FixedDef{
                total_size: 8,
                exp_mag: 4,
                signed: true
            };
            let corr_fixed = fd.from_fp_rounded(f.into()).unwrap();
            prop_assert_eq!(corr_fixed.to_u64().unwrap() >> 4, 7);

        }

        #[test]
        fn overflow_signed_lo(f in -255_f64..-8.){
            let fd = FixedDef{
                total_size: 8,
                exp_mag: 4,
                signed: true
            };
            let corr_fixed = fd.from_fp_rounded(f.into()).unwrap();
            prop_assert_eq!(corr_fixed.to_u64().unwrap() >> 4, 8);

        }


    }
}
