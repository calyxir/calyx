// abstractions for describing representation(s) of numbers

use baa::BitVecOps;

use crate::numimpl::{self};
use crate::typing::*;

// TODO: the below is probably quite bad but. works

impl TypeSpec {
    pub fn read_string(
        &self,
        s: String,
        _end: Endian,
    ) -> Result<baa::BitVecValue, ReadStringErr> {
        if numimpl::is_hexstring(&s) {
            return numimpl::read_hexstring(&s, _end, self.width);
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
                return Err(ReadStringErr::from(format!("unknown {u}")));
            }
        };
        debug_assert!(r.width() <= self.width as u32);
        Ok(r)
    }

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

    pub fn write_hexstring(&self, b: baa::BitVecValue, _end: Endian) -> String {
        b.to_hex_str()
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
    pub(self) data: Vec<baa::BitVecValue>, // container for the data elements
    pub dimensions: [usize; 4], // begrudgingly, multi-dimensional memories are supported
    pub num_dimensions: usize,
    // [dtype] is not stored with every element of the data vec for space efficiency
    dtype: TypeSpec,
    pub end: Endian,
}

impl SingleMem {
    // horrible, but hopefully makes 'implicit' bitcasts much harder and thus more annoying: having to recreate the struct hopefully discourages it
    pub fn new(
        data: Vec<baa::BitVecValue>,
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
    pub fn iter_data<'a>(&'a self) -> std::slice::Iter<'a, baa::BitVecValue> {
        self.data.iter()
    }

    pub fn ty(&self) -> TypeSpec {
        self.dtype.clone()
    }

    /// tries to truncate the input to a certain number of bits.
    pub fn truncate(&mut self, num_bits: usize) -> Result<(), CheckedConvErr> {
        if num_bits > self.dtype.width {
            Err(String::from("truncation to size larger than input"))
        } else if num_bits == self.dtype.width {
            // effectively nops
            self.dtype.class = TypeClass::Bits;
            Ok(())
        } else {
            self.dtype.class = TypeClass::Bits;
            self.dtype.width = num_bits;

            for e in self.data.iter_mut() {
                *e = e.slice(num_bits as u32, 0);
            }
            Ok(())
        }
    }

    pub fn sign_extend(
        &mut self,
        num_bits: usize,
    ) -> Result<(), CheckedConvErr> {
        if num_bits < self.dtype.width {
            Err(String::from(
                "trying to sign-extend to width less than current width. use truncate instead.",
            ))
        } else if num_bits == self.dtype.width {
            // effectively nops
            self.dtype.class = TypeClass::Bits;
            Ok(())
        } else {
            for e in self.data.iter_mut() {
                *e = e.sign_extend((num_bits - self.dtype.width) as u32);
            }
            Ok(())
        }
    }

    pub fn bitcast(&mut self, out_t: TypeSpec) -> Result<(), CheckedConvErr> {
        if out_t.width < self.dtype.width {
            Err(String::from(
                "attempted bitcast to width smaller than current size. use a truncate first if this is intended.",
            ))
        } else {
            self.dtype = out_t;
            Ok(())
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
