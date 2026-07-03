use std::collections::HashMap;

use crate::{numimpl::OpFnTypes::Nop, numrep::*};
use fixed::{FixedI32, FixedI64, FixedU32, FixedU64};

pub struct TypeProps {
    pub from_string_rounding: fn(s: String, end: Endian) -> BinRep,
    pub to_str: fn(b: &BinRep, end: Endian) -> String,
}

type SpecID = u32;

pub enum OpTypes {
    Truncate,
    Bitcast,
    SignExtend,
    Cast(OpCastTypes),
}

#[derive(PartialEq, Eq, Hash)]
pub enum OpCastTypes {
    SatCast,
    WrapCast,
    OverflowCast,
    LosslessCast,
}

pub type OpError = String;

#[derive(Clone)]
pub enum OpFnTypes {
    Falliable(fn(&BinRep, &TypeSpec, &TypeSpec) -> Result<BinRep, OpError>),
    Infalliable(fn(&BinRep, &TypeSpec, &TypeSpec) -> BinRep),
    Nop,
}

#[derive(Default)]
pub struct TypePropsMap {
    // hashmap will be Very Bad and insufficiently general to begin with, but is a start
    lookup: HashMap<TypeSpec, SpecID>,

    m: HashMap<TypeSpec, TypeProps>,

    // mapping of (in_t, out_t, cast_t) to a function
    pub casts: HashMap<(SpecID, SpecID, OpCastTypes), OpFnTypes>,
}

impl TypePropsMap {
    pub fn init(&mut self) {
        self.m.insert(
            TypeSpec {
                width: 32,
                signed: false,
                class: TypeClass::Float,
            },
            TypeProps {
                from_string_rounding: crate::numimpl::float32_str_lossy,
                to_str: crate::numimpl::float32_to_st,
            },
        );
        self.m.insert(
            TypeSpec {
                width: 32,
                signed: false,
                class: TypeClass::Int,
            },
            TypeProps {
                from_string_rounding: crate::numimpl::u32_str_lossy,
                to_str: crate::numimpl::u32_to_st,
            },
        );
    }

    pub fn get_props(&self, t: &TypeSpec) -> &TypeProps {
        self.m.get(t).unwrap()
    }

    fn get_spec_id(&self, t: &TypeSpec) -> SpecID {
        *self.lookup.get(t).unwrap()
    }

    pub fn fun_for_op(
        &self,
        itype: &TypeSpec,
        otype: &TypeSpec,
        op: OpTypes,
    ) -> Result<OpFnTypes, OpError> {
        match op {
            OpTypes::Truncate => {
                if itype.width == otype.width {
                    Ok(OpFnTypes::Nop)
                } else if itype.width > otype.width {
                    Ok(OpFnTypes::Infalliable(generic_truncate))
                } else {
                    Err(String::from("truncate on bad widths"))
                }
            }
            OpTypes::Bitcast => {
                if itype.width <= otype.width {
                    Ok(OpFnTypes::Nop)
                } else {
                    Err(String::from("truncate on bad widths"))
                }
            }
            OpTypes::SignExtend => {
                if itype.width == otype.width {
                    Ok(OpFnTypes::Nop)
                } else if itype.width < otype.width {
                    // sign extension happens regardless of otype signedness
                    if itype.signed {
                        Ok(OpFnTypes::Infalliable(generic_signextend))
                    } else {
                        Err(String::from("tried to sign-extend unsigned"))
                    }
                } else {
                    Err(String::from("truncate on bad widths"))
                }
            }
            OpTypes::Cast(c) => {
                let c = self.casts.get(&(
                    self.get_spec_id(itype),
                    self.get_spec_id(otype),
                    c,
                ));
                match c {
                    Some(f) => Ok(f.clone()),
                    None => Err(String::from("cast not implemented")),
                }
            }
        }
    }

    // preserve dimensions, endianness
    // probably some unnecessary allocs here, but alas
    pub fn checked_apply(
        &self,
        in_mem: SingleMem,
        out_type: &TypeSpec,
        op: OpTypes,
    ) -> Result<SingleMem, CheckedConvErr> {
        let opfun = self.fun_for_op(&in_mem.dtype, out_type, op)?;
        let out_data = match opfun {
            OpFnTypes::Nop => {
                // modify type in place and return
                // return with only type changed

                return Ok(SingleMem {
                    dtype: out_type.clone(),
                    ..in_mem
                });
            }
            OpFnTypes::Falliable(f) => in_mem
                .iter_data()
                .map(|e| f(e, &in_mem.dtype, out_type))
                .collect::<Result<_, _>>()?,
            OpFnTypes::Infalliable(f2) => in_mem
                .iter_data()
                .map(|e| f2(e, &in_mem.dtype, out_type))
                .collect(),
        };
        Ok(SingleMem {
            data: out_data,
            dtype: out_type.clone(),
            ..in_mem
        })
    }
}

pub fn generic_truncate(
    inp: &BinRep,
    _itype: &TypeSpec,
    otype: &TypeSpec,
) -> BinRep {
    let mask = crate::util::mask_n_bits(otype.width);
    inp & mask
}

pub fn generic_signextend(
    inp: &BinRep,
    itype: &TypeSpec,
    otype: &TypeSpec,
) -> BinRep {
    let msb = 1 << (itype.width - 1);
    if msb & inp != 0 {
        let mask1 = crate::util::mask_n_bits(itype.width);
        let mask2 = crate::util::mask_n_bits(otype.width);
        *inp | (!mask1 & mask2)
    } else {
        *inp
    }
}

pub struct IntProps;

pub fn try_from_bytes(
    b: &[u8],
    len: usize,
    width: usize,
    end: Endian,
) -> Result<BinRep, CheckedConvErr> {
    let padded_bytes = crate::util::pad_bytes::<8>(b, len);
    let r = match end {
        Endian::Little => u64::from_le_bytes(padded_bytes),
        Endian::Big => u64::from_be_bytes(padded_bytes),
    };

    // mask to WIDTH bits
    Ok(r & crate::util::mask_n_bits(width))
}

pub fn float32_str_lossy(s: String, _end: Endian) -> BinRep {
    s.parse::<f32>().unwrap().to_bits() as u64
}
pub fn float32_to_st(b: &BinRep, _end: Endian) -> String {
    format!("{}", f32::from_bits(*b as u32))
}

pub fn float64_str_lossy(s: String, _end: Endian) -> BinRep {
    s.parse::<f64>().unwrap().to_bits() as u64
}
pub fn float64_to_st(b: &BinRep, _end: Endian) -> String {
    format!("{}", f64::from_bits(*b))
}

pub fn u32_str_lossy(s: String, _end: Endian) -> BinRep {
    s.parse::<u32>().unwrap() as u64
}
pub fn u32_to_st(b: &BinRep, _end: Endian) -> String {
    format!("{}", *b as u32)
}

pub fn i32_str_lossy(s: String, _end: Endian) -> BinRep {
    s.parse::<i32>().unwrap() as u64
}
pub fn i32_to_st(b: &BinRep, _end: Endian) -> String {
    format!("{}", *b as i32)
}

pub fn u64_str_lossy(s: String, _end: Endian) -> BinRep {
    s.parse::<u64>().unwrap() as u64
}
pub fn u64_to_st(b: &BinRep, _end: Endian) -> String {
    format!("{}", *b as u64)
}

pub fn i64_str_lossy(s: String, _end: Endian) -> BinRep {
    s.parse::<i64>().unwrap() as u64
}
pub fn i64_to_st(b: &BinRep, _end: Endian) -> String {
    format!("{}", *b as i64)
}

pub fn bits_str_lossy(s: String, _end: Endian) -> BinRep {
    s.parse::<u64>().unwrap()
}
pub fn bits_to_st(b: &BinRep, _end: Endian) -> String {
    format!("{:#x}", *b)
}

pub fn ifixed32_string_lossy(s: &str, _end: Endian) -> BinRep {
    let flot = s.parse::<f64>().unwrap();
    (FixedI32::<fixed::types::extra::U16>::from_num(flot).to_bits()) as u64
}
pub fn ifixed32_to_st(b: &BinRep, _end: Endian) -> String {
    let f = FixedI32::<fixed::types::extra::U16>::from_bits(*b as i32);
    format!("{}", f.to_string())
}

pub fn ufixed32_string_lossy(s: &str, _end: Endian) -> BinRep {
    let flot = s.parse::<f64>().unwrap();
    (FixedU32::<fixed::types::extra::U16>::from_num(flot).to_bits()) as u64
}
pub fn ufixed32_to_st(b: &BinRep, _end: Endian) -> String {
    let f = FixedU32::<fixed::types::extra::U16>::from_bits(*b as u32);
    format!("{}", f.to_string())
}

pub fn ifixed64_string_lossy(s: &str, _end: Endian) -> BinRep {
    let flot = s.parse::<f64>().unwrap();
    (FixedI64::<fixed::types::extra::U32>::from_num(flot).to_bits()) as u64
}
pub fn ifixed64_to_st(b: &BinRep, _end: Endian) -> String {
    let f = FixedI64::<fixed::types::extra::U32>::from_bits(*b as i64);
    format!("{}", f.to_string())
}

pub fn ufixed64_string_lossy(s: &str, _end: Endian) -> BinRep {
    let flot = s.parse::<f64>().unwrap();
    (FixedU64::<fixed::types::extra::U32>::from_num(flot).to_bits()) as u64
}
pub fn ufixed64_to_st(b: &BinRep, _end: Endian) -> String {
    let f = FixedU64::<fixed::types::extra::U32>::from_bits(*b as u64);
    format!("{}", f.to_string())
}

#[cfg(test)]

mod tests {
    // use crate::numrep::ReprType;

    // use super::*;

    // #[test]
    // fn test_fixed_from_string() {
    //     let result = IFixed32E16::from_string_lossy("-0.5", nr::Endian::Little);
    //     let t: u32 = (result & 0xffff_ffff) as u32;

    //     // test by getting bits from 0.5 float directly using fixed
    //     let equiv = IFixed32E16::try_from_bytes(
    //         &FixedI32::<fixed::types::extra::U16>::from_num(-0.5).to_le_bytes(),
    //         4,
    //         nr::Endian::Little,
    //     )
    //     .unwrap();
    //     assert_eq!((equiv & 0xffff_ffff) as u32, t)
    // }

    // #[test]
    // fn test_fixed_roundtrip() {
    //     // attempt to roundtrip a value in bytes through the fixed expression
    //     let orig_bits =
    //         FixedI32::<fixed::types::extra::U16>::from_num(-0.5).to_bits();

    //     let thru_bits = IFixed32E16::try_from_bytes(
    //         &orig_bits.to_le_bytes(),
    //         4,
    //         nr::Endian::Little,
    //     )
    //     .unwrap();

    //     let out_bits = (thru_bits & 0xffff_ffff) as i32;

    //     assert_eq!(out_bits, orig_bits);
    // }

    // #[test]
    // fn test_fixed_to_string() {
    //     let equiv = IFixed32E16::try_from_bytes(
    //         &FixedI32::<fixed::types::extra::U16>::from_num(-0.5).to_le_bytes(),
    //         4,
    //         nr::Endian::Little,
    //     )
    //     .unwrap();
    //     assert_eq!(IFixed32E16::to_str(&equiv, nr::Endian::Little), "-0.5");
    // }

    // #[test]
    // fn test_float_roundtrip() {
    //     let orig_bytes = (0.75_f64).to_le_bytes();

    //     let thru_bits =
    //         Float64::try_from_bytes(&orig_bytes, 8, nr::Endian::Little)
    //             .unwrap();

    //     assert_eq!(thru_bits, u64::from_le_bytes(orig_bytes));
    // }

    // #[test]
    // fn test_float_from_string() {
    //     use std::str::FromStr;
    //     let ref_bits = f32::from_str("0.123").unwrap().to_bits();

    //     let comp_bits = Float32::from_string_lossy("0.123", nr::Endian::Little);
    //     assert_eq!(ref_bits as u64, comp_bits);
    // }

    // #[test]
    // fn test_float_to_string() {
    //     let orig_bytes = (0.752_f64).to_le_bytes();

    //     let equiv = Float64::try_from_bytes(&orig_bytes, 8, nr::Endian::Little)
    //         .unwrap();
    //     assert_eq!(Float64::to_str(&equiv, nr::Endian::Little), "0.752");
    // }

    // #[test]
    // fn test_int_roundtrip() {
    //     let orig_bytes = (-1_i64).to_le_bytes();

    //     let thru_bits =
    //         Int64::try_from_bytes(&orig_bytes, 8, nr::Endian::Little).unwrap();

    //     assert_eq!(thru_bits, u64::from_le_bytes(orig_bytes));
    // }

    // #[test]
    // fn test_int_from_string() {
    //     let ref_bits = u64::from_str_radix("ffffffff", 16).unwrap();
    //     let comp_bits =
    //         UInt32::from_string_lossy("4294967295", nr::Endian::Little);
    //     assert_eq!(ref_bits as u64, comp_bits);
    // }

    // #[test]
    // fn test_int_to_string() {
    //     let orig_bytes = (-345_i64).to_le_bytes();
    //     let equiv =
    //         Int64::try_from_bytes(&orig_bytes, 8, nr::Endian::Little).unwrap();
    //     assert_eq!(Int64::to_str(&equiv, nr::Endian::Little), "-345");
    // }
}
