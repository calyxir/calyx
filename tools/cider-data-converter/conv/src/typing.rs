use baa::BitVecValue;
use std::{
    num::{ParseFloatError, ParseIntError},
    str::FromStr,
};

pub enum OpTypes {
    Truncate,
    Bitcast,
    SignExtend,
    // Cast(OpCastTypes),
}

pub type OpError = String;

impl FromStr for OpTypes {
    type Err = OpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trunc" => Ok(OpTypes::Truncate),
            "bitcast" => Ok(OpTypes::Bitcast),
            "sgn-ext" => Ok(OpTypes::SignExtend),
            _ => Err(format!("unknown op {}", s)),
        }
    }
}

/// enum of possible functions between types
/// the full array is provided so some calculations can be easily amortised across the iterations
#[derive(Clone)]
pub enum OpFnTypes {
    Falliable(
        fn(&BitVecValue, &TypeSpec, &TypeSpec) -> Result<BitVecValue, OpError>,
    ),
    Infalliable(fn(&BitVecValue, &TypeSpec, &TypeSpec) -> BitVecValue),
    Nop,
}

/// general, larger 'groups' of types, of which a specific number of bits / signedness is a variant
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub enum TypeClass {
    #[default]
    Bits,
    Int,
    Float,
    Fixed {
        exp_mag: i32, // {fixed_val} = (Binrep) * (2^ (-exp_mag))
    },
    Unknown(usize), // just needs to contain something for future expansion
}

impl std::fmt::Display for TypeClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

// types are instances of typespec rather than traits
// TODO: add guarded constructor which prevents widths larger than 64
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TypeSpec {
    pub width: usize,
    pub signed: bool,
    pub class: TypeClass,
}

/*
    endianness conversions can happen without knowledge of type, so it's in a separate field
*/
#[derive(Clone, Debug, Default)]
pub enum Endian {
    #[default]
    Little,
    Big,
}

// needs a direction, and then enum

// TODO: add a from_bytes form?

#[derive(Debug, thiserror::Error)]
pub enum NumParseErr {
    #[error("could not read {0} as hexstring")]
    HexRead(String),
    #[error("bad float {0:?}")]
    Float(#[from] ParseFloatError),
    #[error("bad int {0:?}")]
    Int(#[from] ParseIntError),
    #[error("incorrect width {0} for {1}")]
    Width(usize, TypeClass),
    #[error("baa internal: passed {0}, {1:?} ")]
    Baa(String, baa::ParseIntError),
    #[error("unknown typeclass: {0}")]
    UnknownType(usize),
    #[error("misc: {0}")]
    Misc(String),
}
