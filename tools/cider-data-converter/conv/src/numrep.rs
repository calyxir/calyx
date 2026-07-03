/// relevant Stuff for describing the representation(s) of numbers

// when printing / writing out binary, use Untypednum.as_bytes() or similar
pub type BinRep = u64;

#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub enum TypeClass {
    #[default]
    Bits,
    Int,
    Float,
    Fixed {
        exp_width: usize,
    },
    Unknown(usize),
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
    pub data: Vec<BinRep>, // container for the data elements
    pub dimensions: [usize; 4], // begrudgingly, multi-dimensional memories are supported
    pub num_dimensions: usize,
    // [dtype] is not within every element of the data vec for space efficiency
    pub dtype: TypeSpec,
    pub end: Endian,
}

// pub type In = TypeSpec;

// only permit checked conversions between typespecs with a known conversion function

impl SingleMem {
    pub fn iter_data(&self) -> std::slice::Iter<BinRep> {
        self.data.iter()
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
