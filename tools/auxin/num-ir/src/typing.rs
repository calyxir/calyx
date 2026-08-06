use baa::{BitVecOps, BitVecValue};
use std::{
    num::{ParseFloatError, ParseIntError},
    str::FromStr,
};

use crate::numimpl;

/// Possible operations on stored values.
pub enum OpTypes {
    Truncate,
    Bitcast,
    SignExtend,
    // Cast(OpCastTypes),
}

/// An error caused by an [OpTypes] operation failing
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

/// enum of possible functions between types.
#[derive(Clone)]
pub enum OpFnTypes {
    Falliable(
        fn(&BitVecValue, &TypeSpec, &TypeSpec) -> Result<BitVecValue, OpError>,
    ),
    Infalliable(fn(&BitVecValue, &TypeSpec, &TypeSpec) -> BitVecValue),
    Nop,
}

/// general, larger 'groups' of types, outside of bit width and signedness.
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub enum TypeClass {
    #[default]
    Bits,
    Int,
    Float,
    /// ``fixed_val = (Binrep) * (2^ (-exp_mag))``
    Fixed {
        exp_mag: i32,
    },
    Unknown(usize), // just needs to contain something for future expansion
}

// impl std::fmt::Display for TypeClass {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_fmt(format_args!("{:?}", self))
//     }
// }

// types are instances of typespec rather than traits
/// A specific type of ``class``.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TypeSpec {
    pub width: usize,
    pub signed: bool,
    pub class: TypeClass,
}
impl TypeSpec {
    /// Attempt to read ``s`` into a [BitVecValue] given the current [TypeSpec]
    pub fn read_str(
        &self,
        s: &str,
        _end: Endian,
    ) -> Result<baa::BitVecValue, NumParseErr> {
        if numimpl::is_hexstring(s) {
            return numimpl::read_hexstring(s, _end, self.width);
        }
        let r = match self.class {
            TypeClass::Bits => numimpl::bits_read(s, _end, self.width)?,
            TypeClass::Int => {
                numimpl::int_read(s, _end, self.width, self.signed)?
            }
            TypeClass::Float => numimpl::float_read(s, _end, self.width)?,
            TypeClass::Fixed { exp_mag } => {
                numimpl::fixed_read(s, _end, self.width, self.signed, exp_mag)?
            }
            TypeClass::Unknown(u) => {
                return Err(NumParseErr::UnknownType(u));
            }
        };
        debug_assert!(r.width() == self.width as u32);
        Ok(r)
    }

    /// Interpret ``b`` as containing information of the current [TypeSpec]
    pub fn write_string(&self, b: &baa::BitVecValue, _end: Endian) -> String {
        debug_assert!(b.width() <= self.width as u32);

        match self.class {
            TypeClass::Bits => numimpl::bits_write(b, _end),
            TypeClass::Int => {
                numimpl::int_write(b, _end, self.width, self.signed)
            }
            TypeClass::Float => numimpl::float_write(b, _end, self.width),
            TypeClass::Fixed { exp_mag } => {
                numimpl::fixed_write(b, _end, self.width, self.signed, exp_mag)
            }
            TypeClass::Unknown(_) => {
                panic!("unimplemented write type")
            }
        }
    }

    /// write out the hexadecimal representation of ``b``
    pub fn write_hexstring(
        &self,
        b: &baa::BitVecValue,
        _end: Endian,
    ) -> String {
        format!("0x{}", b.to_hex_str())
    }

    // try from bytes should be a 'blanket' part of TypeSpec
    /// return number of bytes required to store a value of ``&self``
    pub fn num_bytes(&self) -> usize {
        self.width.div_ceil(8)
    }
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

/// Errors associated with reading a number in
#[derive(Debug, thiserror::Error)]
pub enum NumParseErr {
    #[error("could not read {0} as hexstring")]
    HexRead(String),
    #[error("bad float {0:?}")]
    Float(#[from] ParseFloatError),
    #[error("bad int {0:?}")]
    Int(#[from] ParseIntError),
    #[error("incorrect width {0} for {1:?}")]
    Width(usize, TypeClass),
    #[error("baa internal: passed {0}, {1:?} ")]
    Baa(String, baa::ParseIntError),
    #[error("unknown typeclass: {0}")]
    UnknownType(usize),
    #[error("misc: {0}")]
    Misc(String),
}

#[cfg(test)]
mod tests {
    use crate::typing::*;

    #[test]
    fn read_int() {
        let t = TypeSpec {
            width: 32,
            signed: false,
            class: TypeClass::Bits,
        };
        let e = t.read_str("0x1234", Endian::Little).unwrap();
        println!("{}", e.width());
    }
}
