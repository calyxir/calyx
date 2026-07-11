use std::str::FromStr;

use crate::numimpl::{self};

/// relevant Stuff for describing the representation(s) of numbers

// when printing / writing out binary, use Untypednum.as_bytes() or similar
pub type BinRep = u64;

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
#[derive(Clone)]
pub enum OpFnTypes {
    Falliable(fn(&BinRep, &TypeSpec, &TypeSpec) -> Result<BinRep, OpError>),
    Infalliable(fn(&BinRep, &TypeSpec, &TypeSpec) -> BinRep),
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

// TODO: the below is probably quite bad but. works

impl TypeSpec {
    pub fn read_string(
        &self,
        s: String,
        _end: Endian,
    ) -> Result<BinRep, ReadStringErr> {
        if numimpl::is_hexstring(&s) {
            return numimpl::read_hexstring(&s, _end, self.width);
        }
        let r = match self.class {
            TypeClass::Bits => numimpl::bits_read(s, _end)?,
            TypeClass::Int => {
                numimpl::int_read(s, _end, self.width, self.signed)?
            }
            TypeClass::Float => numimpl::float_read(s, _end, self.width)?,
            TypeClass::Fixed { exp_mag } => {
                numimpl::fixed_read(s, _end, self.width, self.signed, exp_mag)?
            }
            TypeClass::Unknown(u) => {
                return Err(ReadStringErr::from(format!("unknown {u}")));
            }
        };
        debug_assert!(r & !crate::util::mask_n_bits(self.width) == 0);
        Ok(r)
    }

    pub fn write_string(&self, b: BinRep, _end: Endian) -> String {
        debug_assert!(b & !crate::util::mask_n_bits(self.width) == 0);

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

    pub fn write_hexstring(&self, b: BinRep, _end: Endian) -> String {
        format!("{:#x}", b)
    }

    // try from bytes should be a 'blanket' part of TypeSpec
    pub fn num_bytes(&self) -> usize {
        self.width.div_ceil(8)
    }
}

pub type CheckedConvErr = String;

/// [SingleMem] contains the contents of a memory.
#[derive(Debug)]
pub struct SingleMem {
    pub(self) data: Vec<BinRep>, // container for the data elements
    pub dimensions: [usize; 4], // begrudgingly, multi-dimensional memories are supported
    pub num_dimensions: usize,
    // [dtype] is not stored with every element of the data vec for space efficiency
    dtype: TypeSpec,
    pub end: Endian,
}

impl SingleMem {
    // horrible, but hopefully makes 'implicit' bitcasts much harder and thus more annoying: having to recreate the struct hopefully discourages it
    pub fn new(
        data: Vec<BinRep>,
        dimensions: [usize; 4],
        num_dimensions: usize,
        dtype: TypeSpec,
        end: Endian,
    ) -> Self {
        SingleMem {
            data,
            dimensions,
            num_dimensions,
            dtype,
            end,
        }
    }
    pub fn iter_data<'a>(&'a self) -> std::slice::Iter<'a, BinRep> {
        self.data.iter()
    }

    pub fn ty(&self) -> TypeSpec {
        self.dtype.clone()
    }

    /// tries to truncate the input to a certain number of bits.
    pub fn truncate(&mut self, num_bits: usize) -> Result<(), CheckedConvErr> {
        if num_bits > self.dtype.width {
            return Err(String::from("truncation to size larger than input"));
        } else if num_bits == self.dtype.width {
            // effectively nops
            self.dtype.class = TypeClass::Bits;
            return Ok(());
        } else {
            let mask = crate::util::mask_n_bits(num_bits);
            self.dtype.class = TypeClass::Bits;
            self.dtype.width = num_bits;

            for e in self.data.iter_mut() {
                *e &= mask;
            }
            return Ok(());
        }
    }

    pub fn sign_extend(
        &mut self,
        num_bits: usize,
    ) -> Result<(), CheckedConvErr> {
        if num_bits > 64 {
            return Err(String::from(
                "attempted sign extension to size larger than currently-supported bit representation",
            ));
        } else if num_bits < self.dtype.width {
            return Err(String::from(
                "trying to sign-extend to width less than current width. use truncate instead.",
            ));
        } else if num_bits == self.dtype.width {
            // effectively nops
            self.dtype.class = TypeClass::Bits;
            return Ok(());
        } else {
            let msb_mask = 1 << (self.dtype.width - 1);
            let sgn_mask = !crate::util::mask_n_bits(self.dtype.width)
                & crate::util::mask_n_bits(num_bits);

            self.dtype.class = TypeClass::Bits;
            self.dtype.width = num_bits;

            for e in self.data.iter_mut() {
                if msb_mask & *e != 0 {
                    *e |= sgn_mask;
                }
            }
            return Ok(());
        }
    }

    pub fn bitcast(&mut self, out_t: TypeSpec) -> Result<(), CheckedConvErr> {
        if out_t.width < self.dtype.width {
            return Err(String::from(
                "attempted bitcast to width smaller than current size. use a truncate first if this is intended.",
            ));
        } else {
            self.dtype = out_t;
            return Ok(());
        }
    }

    pub fn apply_opfun(
        self,
        opfun: OpFnTypes,
        out_type: &TypeSpec,
    ) -> Result<SingleMem, CheckedConvErr> {
        let out_data = match opfun {
            OpFnTypes::Nop => {
                // modify type in place and return
                // return with only type changed

                return Ok(SingleMem { ..self });
            }
            OpFnTypes::Falliable(f) => self
                .iter_data()
                .map(|e| f(e, &self.dtype, out_type))
                .collect::<Result<_, _>>()?,
            OpFnTypes::Infalliable(f2) => self
                .iter_data()
                .map(|e| f2(e, &self.dtype, out_type))
                .collect(),
        };
        Ok(SingleMem {
            data: out_data,
            dtype: out_type.clone(),
            ..self
        })
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
