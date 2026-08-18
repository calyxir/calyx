//! Types and structures for assigning a 'type' to binary information

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
#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("PLACEHOLDER: op failed")]
    OpFailed,

    #[error("can't truncate to size larger than input")]
    TruncWider,

    #[error("can't sign-extend to width less than input")]
    SENarrower,

    #[error("bitcast to width smaller than current size")]
    BitcastNarrower,
}

impl FromStr for OpTypes {
    type Err = OpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trunc" => Ok(OpTypes::Truncate),
            "bitcast" => Ok(OpTypes::Bitcast),
            "sgn-ext" => Ok(OpTypes::SignExtend),
            _ => Err(OpError::OpFailed),
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
    /// refers to ieee754. implementaion relies onRust's ``f32`` and ``f64``, custom mantissa / exponent configurations are unsupported.
    Float,
    /// ``fixed_val = (Binrep) * (2^ (-exp_mag))``
    Fixed {
        exp_mag: i32,
    },
    /// A placeholder for future custom type handling. Attempting to use this with other library functions will currently return an error.
    Unknown(usize), // just needs to contain something for future expansion
}

#[cfg(feature = "rand1")]
/// generate a random, valid typeclass
///
/// since fixed-point magnitude is constrained by the width of its containing type, it is set to zero.
pub fn rand_class(rng: &mut impl rand::Rng) -> TypeClass {
    use rand::RngExt;

    let class_choice = rng.random_range(0..4);
    match class_choice {
        0 => TypeClass::Bits,
        1 => TypeClass::Int,
        2 => TypeClass::Float,
        3 => TypeClass::Fixed { exp_mag: 0 },
        _ => panic!("chose random that's not in [0,1,2,3]"),
    }
}

// types are instances of typespec rather than traits
/// A specific type of a [TypeClass].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TypeSpec {
    pub width: usize,
    /// Signedness (usually two's complement). Disregarded for floating-point and bits [TypeClass]es
    pub signed: bool,
    pub class: TypeClass,
}
impl TypeSpec {
    /// Attempt to read ``s`` into a [BitVecValue] given the current [TypeSpec]
    ///
    /// If ``s`` is a hex string (i.e. a sequence of chracters 0-f prefixed by ``0x``), ``self.width`` bits will be read in and interpreted as ``self``'s type.
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

    /// Interpret ``b`` as containing information of the current [TypeSpec], and return a string containing the corresponding information.
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

    /// write out the hexadecimal representation of ``b``, prefixed by ``0x``
    pub fn write_hexstring(
        &self,
        b: &baa::BitVecValue,
        _end: Endian,
    ) -> String {
        format!("0x{}", b.to_hex_str())
    }

    /// return number of bytes required to store a value of ``&self``
    pub fn num_bytes(&self) -> usize {
        self.width.div_ceil(8)
    }
}

#[cfg(feature = "rand1")]
/// create a random, valid [TypeSpec]
pub fn rand_type(rng: &mut impl rand::Rng) -> TypeSpec {
    use rand::RngExt;

    let mut class = rand_class(rng);
    let width: usize = match &mut class {
        TypeClass::Float => {
            if rng.random_bool(0.5) {
                64
            } else {
                32
            }
        }
        TypeClass::Fixed { exp_mag: e } => {
            let w: usize = rng.random_range(0..128);
            *e = rng.random_range(0..w) as i32;
            w
        }
        _ => rng.random_range(0..128),
    };
    TypeSpec {
        width,
        signed: rng.random_bool(0.5),
        class,
    }
}

/// Endianness.
///
/// No functions currently implement support for endianness, but it is included for future use.
///
/// Endian conversions can happen independent of type, so it is separated from [TypeClass]
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
    #[error("provided string too large for {0}")]
    TooLarge(usize), // TODO: this is a temporary patch over deficient baa functionality. patch alter.
    #[error("unknown typeclass: {0}")]
    UnknownType(usize),
    #[error("misc: {0}")]
    Misc(String),
}

#[cfg(test)]
mod tests {
    use crate::typing::*;
    use proptest::prelude::*;

    // test serialisation / deserialisation for each class of type
    proptest! {
        #[test]
        fn int_unsign_roundtrip(s in any::<u64>()){
            let u64_spec = TypeSpec{
                width: 64,
                signed: false,
                class: TypeClass::Int
            };

            let u32_spec = TypeSpec{
                width: 32,
                signed: false,
                class: TypeClass::Int
            };

            let sform = format!("{}", s);
            let i = u64_spec.read_str(&sform, Endian::Little).unwrap();
            let res = u64_spec.write_string(&i, Endian::Little);
            prop_assert_eq!(res, sform.clone());
            if s > (u32::MAX.into()){
                prop_assert!(u32_spec.read_str(&sform, Endian::Little).is_err());
            }
        }


        #[test]
        fn int_sign_roundtrip(s in any::<i64>()){
            let i64_spec = TypeSpec{
                width: 64,
                signed: true,
                class: TypeClass::Int
            };

            let i32_spec = TypeSpec{
                width: 32,
                signed: true,
                class: TypeClass::Int
            };

            let sform = format!("{}", s);
            let i = i64_spec.read_str(&sform, Endian::Little).unwrap();
            let res = i64_spec.write_string(&i, Endian::Little);
            prop_assert_eq!(res, sform.clone());
            if s > (i32::MAX.into()){
                prop_assert!(i32_spec.read_str(&sform, Endian::Little).is_err());
            }
        }


        #[test]
        fn float_roundtrip(s in any::<f64>()){
            let f64_spec = TypeSpec{
                width: 64,
                signed: false,
                class: TypeClass::Float
            };

            let sform = format!("{}", s);
            let i = f64_spec.read_str(&sform, Endian::Little).unwrap();
            let res = f64_spec.write_string(&i, Endian::Little);
            prop_assert_eq!(res, sform.clone());

        }

        #[test]
        fn bits_roundtrip(s in any::<u32>()){


            let u32_spec = TypeSpec{
                width: 32,
                signed: false,
                class: TypeClass::Bits
            };

            let sform = format!("{:#034b}", s); // 32 + the '0b'
            let i = u32_spec.read_str(&sform, Endian::Little).unwrap();
            let res = u32_spec.write_string(&i, Endian::Little);
            prop_assert_eq!(res, sform);

        }

    }
}
