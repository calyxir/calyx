/// relevant Stuff for describing the representation(s) of numbers

// when printing / writing out binary, use Untypednum.as_bytes() or similar
pub type BinRep = u64;

#[derive(Debug)]
pub struct DataSet<T: ReprType + Sized> {
    pub data: Vec<BinRep>, // container for the data elements
    pub dimensions: [usize; 4], // begrudgingly, multi-dimensional memories are supported
    pub num_dimensions: usize,
    // [dtype] is not within every element of the data vec for space efficiency
    pub dtype: std::marker::PhantomData<T>,
    pub end: Endian,
}

pub trait DataTrait {
    fn test_f(self) -> Box<dyn DataTrait>;
}

impl<T: ReprType> DataTrait for DataSet<T> {}

// probably some unnecessary allocs here, but alas
impl<T: ReprType> DataSet<T> {
    pub fn checked_cast<O: ReprType>(
        &self,
    ) -> Result<DataSet<O>, CheckedConvErr>
    where
        O: CheckedFrom<T>,
    {
        let new_data: Result<Vec<BinRep>, _> =
            self.data.iter().map(|br| O::ckd_from(*br)).collect();
        Ok(DataSet::<O> {
            data: new_data?,
            dimensions: self.dimensions,
            num_dimensions: self.num_dimensions,
            dtype: std::marker::PhantomData::<O>,
            end: self.end.clone(),
        })
    }
}
/*
    endianness conversions can happen without knowledge of type
*/
#[derive(Clone, Debug)]
pub enum Endian {
    Little,
    Big,
}

/*
    DataSets are given type via implementers of the 'ReprType' trait.

    this hopefully permits extensibility, and discourages weird conversions 'through' untyped

*/

#[derive(Clone, Debug)]
pub enum ReprAs {
    Bits,
    Int { signed: bool },
    Float,
    Fixed { signed: bool, exp_width: u32 },
    Unknown(String),
}

pub type CheckedConvErr = String;

pub trait ReprType {
    const WIDTH: usize;
    const NUM_BYTES: usize = Self::WIDTH.div_ceil(8);

    fn repr_as() -> ReprAs;

    /// conversion from bytes is lossless by default.
    /// [len] should generally be the same as NUM_BYTES
    /// it is the responsibility of the implementer to confirm that [len] is correct.
    /// data must be byte aligned.
    /// [BinRep] is distinct from the input bytes [b] to discourage conversions 'through' an untyped
    fn try_from_bytes(
        b: &[u8],
        len: usize,
        end: Endian,
    ) -> Result<BinRep, CheckedConvErr>;

    /// conversion from a string is lossy by default.
    /// hopefully going [b.from_str(a.to_str())] is perverse enough that people realise something is up.
    fn from_string_lossy(s: &str, end: Endian) -> BinRep;

    fn to_str(b: &BinRep, end: Endian) -> String;
}

/// opt-in trait for lossy byte conversion
/// if a conversion cannot take place, these will do some sort of known rounding behaviour
pub trait LossyFromBytes: ReprType {
    fn from_bytes_lossy(b: &[u8], len: usize, end: Endian) -> BinRep;
}

// opt-in trait for lossless string conversion
// attempts to be 'bit precise', treating the string as if it is 'really representable'
pub trait LosslessFromStr: ReprType {
    fn try_from_string(
        s: String,
        end: Endian,
    ) -> Result<BinRep, CheckedConvErr>;
}

/// [ckd_from] should be used for potentially lossy conversions between types
/// [O] is the output type, [I] is the input type (defaults to self)

// not sure if using TryFrom here would be acceptable. in case it breaks things, this is what we use.
pub trait CheckedFrom<O: ReprType, I: ReprType = Self> {
    fn ckd_from(i: BinRep) -> Result<BinRep, CheckedConvErr>;
}

/// given a string, try to infer its type from a set of 'common defaults'
/// integers are put into u64/i64, decimals into f64, hex strings into Bits
/// otherwise, panic
pub enum InferredRes {
    Fail,
    I(crate::numimpl::Int64),
    FP(crate::numimpl::Float64),
}

pub fn TryInferString(
    inp: &String,
) -> Result<(InferredRes, BinRep), CheckedConvErr> {
    unimplemented!()
}
