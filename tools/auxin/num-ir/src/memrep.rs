// abstractions for describing representation(s) of numbers

use crate::typing::*;
use baa::BitVecOps;
use smallvec::SmallVec;

/// [SingleMem] contains the contents of a memory.
///
/// A memory is defined as a set of data with the same type. Modifying memory contents is discouraged, thus contents are hidden.
#[derive(Debug)]
pub struct SingleMem {
    pub(self) data: Vec<baa::BitVecValue>, // container for the data elements
    pub dimensions: SmallVec<[usize; 4]>, // begrudgingly, multi-dimensional memories are supported
    // [dtype] is not stored with every element of the data vec for space efficiency
    dtype: TypeSpec,
    pub end: Endian,
}

impl SingleMem {
    // horrible, but hopefully makes 'implicit' bitcasts much harder and thus more annoying: having to recreate the struct hopefully discourages it
    /// Create a new [SingleMem] from the specified parameters.
    pub fn new(
        data: Vec<baa::BitVecValue>,
        dimensions: SmallVec<[usize; 4]>,
        dtype: TypeSpec,
        end: Endian,
    ) -> Self {
        SingleMem {
            data,
            dimensions,
            dtype,
            end,
        }
    }

    pub fn size(&self) -> usize {
        // self.dimensions.iter().product()
        self.data.len()
    }

    /// return an immutable iterator to the memory contents
    pub fn iter_data<'a>(&'a self) -> std::slice::Iter<'a, baa::BitVecValue> {
        self.data.iter()
    }

    /// return an immutable reference to this memory's type.
    pub fn ty(&self) -> &TypeSpec {
        &self.dtype
    }

    /// tries to truncate the input to ``num_bits``. turns own [TypeClass] into ``Bits``
    pub fn truncate(
        &mut self,
        num_bits: usize,
    ) -> Result<(), crate::typing::OpError> {
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

    /// attempt to sign extend all entries to ``num_bits``. turns own [TypeClass] into ``Bits``
    pub fn sign_extend(
        &mut self,
        num_bits: usize,
    ) -> Result<(), crate::typing::OpError> {
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

    /// Transform own [TypeClass] to ``Bits``
    pub fn bitcast(
        &mut self,
        out_t: TypeSpec,
    ) -> Result<(), crate::typing::OpError> {
        if out_t.width < self.dtype.width {
            Err(String::from(
                "attempted bitcast to width smaller than current size. use a truncate first if this is intended.",
            ))
        } else {
            self.dtype = out_t;
            Ok(())
        }
    }

    /// Apply ``opfun`` to each entry, returning a copy of the memory with the results of the application.
    pub fn apply_opfun(
        self,
        opfun: OpFnTypes,
        out_type: TypeSpec,
    ) -> Result<SingleMem, crate::typing::OpError> {
        let out_data = match opfun {
            OpFnTypes::Nop => {
                // modify type in place and return
                // return with only type changed

                return Ok(SingleMem { ..self });
            }
            OpFnTypes::Falliable(f) => self
                .iter_data()
                .map(|e| f(e, &self.dtype, &out_type))
                .collect::<Result<_, _>>()?,
            OpFnTypes::Infalliable(f2) => self
                .iter_data()
                .map(|e| f2(e, &self.dtype, &out_type))
                .collect(),
        };
        Ok(SingleMem {
            data: out_data,
            dtype: out_type,
            ..self
        })
    }
}

#[cfg(feature = "rand1")]
pub fn rand_dims(num_dims: usize, dim_max: usize) -> SmallVec<[usize; 4]> {
    // generates a random shape with num_dims elements, and with a maximum of dim_max for each dimension
    unimplemented!();
}
