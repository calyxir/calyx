use crate::numrep as nr;
use fixed::{FixedI32, FixedU32};

pub struct Bits<const WIDTH: usize>;

impl<const WIDTH: usize> nr::ReprType for Bits<WIDTH> {
    const WIDTH: usize = WIDTH;
    const NUM_BYTES: usize = Self::WIDTH.div_ceil(8);

    fn repr_as() -> nr::ReprAs {
        nr::ReprAs::Bits
    }

    fn try_from_bytes(
        b: &[u8],
        len: usize,
        end: nr::Endian,
    ) -> Result<nr::BinRep, nr::CheckedConvErr> {
        if len != Self::NUM_BYTES {
            return Err(String::from("not enough bytes!"));
        }
        if len > 8 {
            return Err(String::from(
                "attempting to represent val larger than 64 bits",
            ));
        }
        let padded_bytes = crate::util::pad_bytes::<8>(b, len);

        let r = match end {
            nr::Endian::Little => u64::from_le_bytes(padded_bytes),
            nr::Endian::Big => u64::from_be_bytes(padded_bytes),
        };

        // mask to WIDTH bits
        Ok(r & crate::util::mask_n_bits(Self::WIDTH))
    }
    fn from_string_lossy(s: &str, _end: nr::Endian) -> nr::BinRep {
        s.parse::<u64>().unwrap()
    }
    fn to_str(b: &nr::BinRep, _end: nr::Endian) -> String {
        format!("{:#x}", b)
    }
}

pub struct Int<const WIDTH: usize, const SIGNED: bool>;

macro_rules! impl_int_repr {
    ($int_t: ty, $bytect: literal, $signed: literal) => {
        impl nr::ReprType for Int<{ $bytect * 8 }, { $signed }> {
            const WIDTH: usize = { $bytect * 8 };
            const NUM_BYTES: usize = Self::WIDTH.div_ceil(8);

            fn repr_as() -> nr::ReprAs {
                nr::ReprAs::Int { signed: $signed }
            }

            fn try_from_bytes(
                b: &[u8],
                len: usize,
                end: nr::Endian,
            ) -> Result<nr::BinRep, nr::CheckedConvErr> {
                if len != Self::NUM_BYTES {
                    return Err(String::from("not enough bytes!"));
                }
                if len > 8 {
                    return Err(String::from(
                        "attempting to represent val larger than 64 bits",
                    ));
                }
                let padded_bytes = crate::util::pad_bytes::<$bytect>(b, len);

                let r = match end {
                    nr::Endian::Little => <$int_t>::from_le_bytes(padded_bytes),
                    nr::Endian::Big => <$int_t>::from_be_bytes(padded_bytes),
                } as u64;

                // mask to WIDTH bits
                Ok(r & crate::util::mask_n_bits(Self::WIDTH))
            }
            fn from_string_lossy(s: &str, _end: nr::Endian) -> nr::BinRep {
                s.parse::<$int_t>().unwrap() as u64
            }
            fn to_str(b: &nr::BinRep, _end: nr::Endian) -> String {
                format!("{}", *b as $int_t)
            }
        }
    };
}

pub type UInt32 = Int<32, false>;
impl_int_repr!(u32, 4, false);

pub type UInt64 = Int<64, false>;
impl_int_repr!(u64, 8, false);

pub type Int32 = Int<32, true>;
impl_int_repr!(i32, 4, true);

pub type Int64 = Int<64, true>;
impl_int_repr!(i64, 8, true);

pub struct Float<const WIDTH: usize>;

macro_rules! impl_float_repr {
    ($flot_t: ty, $int_equiv_t: ty, $bytect: literal) => {
        impl nr::ReprType for Float<{ $bytect * 8 }> {
            const WIDTH: usize = { $bytect * 8 };
            const NUM_BYTES: usize = Self::WIDTH.div_ceil(8);

            fn repr_as() -> nr::ReprAs {
                nr::ReprAs::Float
            }

            fn try_from_bytes(
                b: &[u8],
                len: usize,
                end: nr::Endian,
            ) -> Result<nr::BinRep, nr::CheckedConvErr> {
                if len != Self::NUM_BYTES {
                    return Err(String::from("not enough bytes!"));
                }
                if len > 8 {
                    return Err(String::from(
                        "attempting to represent val larger than 64 bits",
                    ));
                }
                let padded_bytes = crate::util::pad_bytes::<$bytect>(b, len);

                let r = match end {
                    nr::Endian::Little => {
                        <$flot_t>::from_le_bytes(padded_bytes).to_bits()
                    }
                    nr::Endian::Big => {
                        <$flot_t>::from_be_bytes(padded_bytes).to_bits()
                    }
                } as u64;

                // mask to WIDTH bits
                Ok(r & crate::util::mask_n_bits(Self::WIDTH))
            }
            fn from_string_lossy(s: &str, _end: nr::Endian) -> nr::BinRep {
                s.parse::<$flot_t>().unwrap().to_bits() as u64
            }
            fn to_str(b: &nr::BinRep, _end: nr::Endian) -> String {
                format!("{}", <$flot_t>::from_bits(*b as $int_equiv_t))
            }
        }
    };
}

pub type Float64 = Float<64>;
impl_float_repr!(f64, u64, 8);

pub type Float32 = Float<32>;
impl_float_repr!(f32, u32, 4);

pub struct WrappedFixed<
    const WIDTH: usize,
    const EXP_BITS: usize,
    const SIGNED: bool,
>;

macro_rules! impl_fixed_repr {
    ($fixed_t: ty, $assoc_t: ty, $bytect: literal, $exp_bits: literal, $signed: literal) => {
        impl nr::ReprType
            for WrappedFixed<{ $bytect * 8 }, $exp_bits, $signed>
        {
            const WIDTH: usize = { $bytect * 8 };
            const NUM_BYTES: usize = Self::WIDTH.div_ceil(8);

            fn repr_as() -> nr::ReprAs {
                nr::ReprAs::Fixed {
                    exp_width: $exp_bits,
                    signed: $signed,
                }
            }

            fn try_from_bytes(
                b: &[u8],
                len: usize,
                end: nr::Endian,
            ) -> Result<nr::BinRep, nr::CheckedConvErr> {
                if len != Self::NUM_BYTES {
                    return Err(String::from("not enough bytes!"));
                }
                if len > 8 {
                    return Err(String::from(
                        "attempting to represent val larger than 64 bits",
                    ));
                }
                let padded_bytes = crate::util::pad_bytes::<$bytect>(b, len);

                let r = match end {
                    nr::Endian::Little => {
                        <$fixed_t>::from_le_bytes(padded_bytes).to_bits()
                    }
                    nr::Endian::Big => {
                        <$fixed_t>::from_be_bytes(padded_bytes).to_bits()
                    }
                } as u64;

                // mask to WIDTH bits
                Ok(r & crate::util::mask_n_bits(Self::WIDTH))
            }
            fn from_string_lossy(s: &str, _end: nr::Endian) -> nr::BinRep {
                let flot = s.parse::<f64>().unwrap();
                (<$fixed_t>::from_num(flot).to_bits()) as u64
            }
            fn to_str(b: &nr::BinRep, _end: nr::Endian) -> String {
                let f: $fixed_t = <$fixed_t>::from_bits(*b as $assoc_t);
                format!("{}", f.to_string())
            }
        }
    };
}

// TODO: crate has signed fixed-point types

pub type UFixed32E16 = WrappedFixed<32, 16, false>;
impl_fixed_repr!(FixedU32<fixed::types::extra::U16>, u32, 4, 16, false);

pub type IFixed32E16 = WrappedFixed<32, 16, true>;
impl_fixed_repr!(FixedI32<fixed::types::extra::U16>, i32, 4, 16, true);

#[cfg(test)]

mod tests {
    use crate::numrep::ReprType;

    use super::*;

    #[test]
    fn test_fixed_from_string() {
        let result = IFixed32E16::from_string_lossy("-0.5", nr::Endian::Little);
        let t: u32 = (result & 0xffff_ffff) as u32;

        // test by getting bits from 0.5 float directly using fixed
        let equiv = IFixed32E16::try_from_bytes(
            &FixedI32::<fixed::types::extra::U16>::from_num(-0.5).to_le_bytes(),
            4,
            nr::Endian::Little,
        )
        .unwrap();
        assert_eq!((equiv & 0xffff_ffff) as u32, t)
    }

    #[test]
    fn test_fixed_roundtrip() {
        // attempt to roundtrip a value in bytes through the fixed expression
        let orig_bits =
            FixedI32::<fixed::types::extra::U16>::from_num(-0.5).to_bits();

        let thru_bits = IFixed32E16::try_from_bytes(
            &orig_bits.to_le_bytes(),
            4,
            nr::Endian::Little,
        )
        .unwrap();

        let out_bits = (thru_bits & 0xffff_ffff) as i32;

        assert_eq!(out_bits, orig_bits);
    }

    #[test]
    fn test_fixed_to_string() {
        let equiv = IFixed32E16::try_from_bytes(
            &FixedI32::<fixed::types::extra::U16>::from_num(-0.5).to_le_bytes(),
            4,
            nr::Endian::Little,
        )
        .unwrap();
        assert_eq!(IFixed32E16::to_str(&equiv, nr::Endian::Little), "-0.5");
    }

    #[test]
    fn test_float_roundtrip() {
        let orig_bytes = (0.75_f64).to_le_bytes();

        let thru_bits =
            Float64::try_from_bytes(&orig_bytes, 8, nr::Endian::Little)
                .unwrap();

        assert_eq!(thru_bits, u64::from_le_bytes(orig_bytes));
    }

    #[test]
    fn test_float_from_string() {
        use std::str::FromStr;
        let ref_bits = f32::from_str("0.123").unwrap().to_bits();

        let comp_bits = Float32::from_string_lossy("0.123", nr::Endian::Little);
        assert_eq!(ref_bits as u64, comp_bits);
    }

    #[test]
    fn test_float_to_string() {
        let orig_bytes = (0.752_f64).to_le_bytes();

        let equiv = Float64::try_from_bytes(&orig_bytes, 8, nr::Endian::Little)
            .unwrap();
        assert_eq!(Float64::to_str(&equiv, nr::Endian::Little), "0.752");
    }

    #[test]
    fn test_int_roundtrip() {
        let orig_bytes = (-1_i64).to_le_bytes();

        let thru_bits =
            Int64::try_from_bytes(&orig_bytes, 8, nr::Endian::Little).unwrap();

        assert_eq!(thru_bits, u64::from_le_bytes(orig_bytes));
    }

    #[test]
    fn test_int_from_string() {
        let ref_bits = u64::from_str_radix("ffffffff", 16).unwrap();
        let comp_bits =
            UInt32::from_string_lossy("4294967295", nr::Endian::Little);
        assert_eq!(ref_bits as u64, comp_bits);
    }

    #[test]
    fn test_int_to_string() {
        let orig_bytes = (-345_i64).to_le_bytes();
        let equiv =
            Int64::try_from_bytes(&orig_bytes, 8, nr::Endian::Little).unwrap();
        assert_eq!(Int64::to_str(&equiv, nr::Endian::Little), "-345");
    }
}
