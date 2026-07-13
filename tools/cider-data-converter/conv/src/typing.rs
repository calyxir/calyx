use baa::BitVecValue;
use std::str::FromStr;

pub enum OpTypes {
    Truncate,
    Bitcast,
    SignExtend,
    // Cast(OpCastTypes),
}

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

pub type OpError = String;

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

// types are instances of typespec rather than traits
// TODO: add guarded constructor which prevents widths larger than 64
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TypeSpec {
    pub width: usize,
    pub signed: bool,
    pub class: TypeClass,
}

#[derive(Debug)]
pub enum ReadStringErr {
    BadValue(String),
}

impl<T: ToString> From<T> for ReadStringErr {
    fn from(value: T) -> Self {
        Self::BadValue(value.to_string())
    }
}
