//! very bad fixed-point, a simple runtime fixed-point library

use baa::{BitVecMutOps, BitVecOps, BitVecValue};

use crate::typing::NumParseErr;

/// a definition for a fixed-point type.
///
/// signed-magnitude fixed-point is not supported. only supports two's complement sign.
///
/// while a [FixedDef] might specify a number with greater range / precision than can be contained within an f64, the included parsing functions only parse to / from f64.
pub struct FixedDef {
    /// Width of whole type
    pub total_size: usize,
    /// Magnitude of exponent. The 'true' value which a set of bits represents is
    /// ``int(bits) * 2**(-exp_mag)``
    /// Negative exponents are possible to represent, but are most likely not going to function well with the provided functions.
    pub exp_mag: i32,

    /// Two's complement sign
    pub signed: bool,
}

// much of the below draws upon https://en.wikipedia.org/wiki/Fixed-point_arithmetic#Conversion_to_and_from_floating-point

impl FixedDef {
    /// obtain the max floating-point (int part) which can be represented by the current fixed-point definition
    pub fn max(&self) -> f64 {
        let mut bits = BitVecValue::ones(self.total_size as u32);

        if self.signed {
            bits.clear_bit(self.total_size as u32 - 1);
            self.to_fp_rounded(&bits)
        } else {
            self.to_fp_rounded(&bits)
        }
    }

    /// obtain the minimum floating-point which can be represented by the current fixed-point definition
    pub fn min(&self) -> f64 {
        if !self.signed {
            0.
        } else {
            let int_len = (self.total_size - self.exp_mag as usize) as i32;

            -f64::powi(2., int_len - 1)
        }
    }

    // TODO: below could accept a closure for rounding / mapping behaviour in from and to directions

    /// Create a fixed-point representation of the value ``inp`` using the current [FixedDef]. The new fixed-point value will attempt to approximate the value of ``inp``, with rounding on even ties.
    ///
    /// Out of bounds behaviour: if the input is too large/small to be represented with the fixed-point, the closest value will be returned (i.e. this function saturates)
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
        let v = in_as_fp.expect("can't fit input to f64");
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

/// given a fixed-point definition and expected/got values, compare expected against obtained value. returns true if the difference between them is within the fixed point specification's precision limit.
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

    // basic test of within precision: 0.7 is not perfectly
    // representable with the fixed-def, but the fixed definition should be pretty close.
    #[test]
    fn within_precision_basic() {
        let expc = 0.7;
        let fd = FixedDef {
            total_size: 32,
            exp_mag: 4,
            signed: false,
        };
        let t = fd.from_fp_rounded(expc).unwrap(); // should be 0.6875
        assert!(within_precision(&fd, expc, &t));

        // .75 is not within the precision limit, so this should be false
        assert!(!within_precision(&fd, 0.75, &t));
    }

    use proptest::prelude::*;
    proptest! {
        // proptest within_precision on a bounded set of numbers and exponent.
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

        // reading an unsigned number which exceeds the integer range should saturate
        #[test]
        fn sat_unsigned(f in 17f32..255.){
            let fd = FixedDef{
                total_size: 8,
                exp_mag: 4,
                signed: false
            };
            let corr_fixed = fd.from_fp_rounded(f.into()).unwrap();
            let corr_bits = corr_fixed.to_u64().unwrap();

            // saturates at maximum uint value
            prop_assert_eq!(corr_bits , 0xff);

        }

        // reading a signed number greater than the max should saturate at highest value
        #[test]
        fn sat_signed_hi(f in 9f32..255.){
            let fd = FixedDef{
                total_size: 8,
                exp_mag: 4,
                signed: true
            };
            let corr_fixed = fd.from_fp_rounded(f.into()).unwrap();
            let corr_bits = corr_fixed.to_u64().unwrap();

            prop_assert_eq!(corr_bits >> 4, 7);
            // all decimal bits should be set
            prop_assert_eq!(corr_bits & 0xf, 0xf);

        }


        // reading a signed number greater than the max should saturate at lowest value
        #[test]
        fn sat_signed_lo(f in -255_f64..-8.){
            let fd = FixedDef{
                total_size: 8,
                exp_mag: 4,
                signed: true
            };
            let corr_fixed = fd.from_fp_rounded(f).unwrap();
            let corr_bits = corr_fixed.to_u64().unwrap();

            prop_assert_eq!(corr_bits>> 4, 8);
            // due to two's complement, fractional bits should be zeroed
            prop_assert_eq!(corr_bits & 0xf, 0);

        }


    }
}
