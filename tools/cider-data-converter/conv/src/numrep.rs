use crate::numimpl::OpFnTypes;

/// relevant Stuff for describing the representation(s) of numbers

// when printing / writing out binary, use Untypednum.as_bytes() or similar
pub type BinRep = u64;

/// general, larger 'groups' of types, of which a specific number of bits / signedness is a variant
#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub enum TypeClass {
    #[default]
    Bits,
    Int,
    Float,
    Fixed {
        exp_width: usize,
    },
    Unknown(usize), // just needs to contain something for future expansion
}

// types are instances of typespec rather than traits
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypeSpec {
    pub width: usize,
    pub signed: bool,
    pub class: TypeClass,
}

// try from bytes should be a 'blanket' part of TypeSpec

impl TypeSpec {
    pub fn num_bytes(&self) -> usize {
        self.width.div_ceil(8)
    }
}

pub struct SingleMem {
    pub(self) data: Vec<BinRep>, // container for the data elements
    pub dimensions: [usize; 4], // begrudgingly, multi-dimensional memories are supported
    pub num_dimensions: usize,
    // [dtype] is not stored with every element of the data vec for space efficiency
    dtype: TypeSpec,
    pub end: Endian,
}

// pub type In = TypeSpec;

// only permit checked conversions between typespecs with a known conversion function

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

    pub fn apply_opfun(
        self,
        opfun: crate::numimpl::OpFnTypes,
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

pub type CheckedConvErr = String;
