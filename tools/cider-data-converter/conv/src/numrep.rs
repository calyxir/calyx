/// relevant Stuff for describing the representation(s) of numbers

// when printing / writing out binary, use Untypednum.as_bytes() or similar
pub type BinRep = u64;

pub struct DataSet<T: ReprType> {
    data: Vec<BinRep>,          // container for the data elements
    pub dimensions: [usize; 4], // begrudgingly, multi-dimensional memories are supported
    pub num_dimensions: usize,
    pub dtype: T, // separate from data for space efficiency
    pub end: Endian,
}

// TODO: impl for checked conversion of DataSet<T> to DataSet<S>: wrap try_conv

/*
    endianness conversions can happen without knowledge of type
*/
pub enum Endian {
    Little,
    Big,
}

/*
    DataSets are given type via implementers of the 'ReprType' trait.

    this hopefully permits extensibility, and discourages weird conversions 'through' untyped

*/

pub type CheckedConvErr = String;

pub trait ReprType {
    const WIDTH: u32;
    const NUM_BYTES: u32 = Self::WIDTH.div_ceil(8);

    /// conversion from bytes is lossless by default.
    /// [len] should generally be the same as NUM_BYTES
    /// it is the responsibility of the implementer to confirm that [len] is correct.
    /// [BinRep] is distinct from the input bytes [b] to discourage conversions 'through' an untyped
    fn try_from_bytes(b: &[u8], len: u32) -> Result<BinRep, CheckedConvErr>;

    /// conversion from a string is lossy by default.
    /// hopefully going [b.from_str(a.to_str())] is perverse enough that people realise something is up.
    fn from_string(s: &str) -> BinRep;

    fn to_str(b: &BinRep) -> String;
}

/// opt-in traits for infalliable conversion.
/// if a conversion cannot take place, these will do some sort of known rounding behaviour

/// opt-in trait for lossy byte conversion
pub trait LossyFromBytes: ReprType {
    fn from_bytes_lossy(b: &[u8], len: u32) -> BinRep;
}

// opt-in trait for lossless string conversion
// attempts to be 'bit precise', treating the string as if it is 'really representable'
pub trait LosslessFromStr: ReprType {
    fn from_string(s: String) -> Result<BinRep, CheckedConvErr>;
}

/// [try_conv] should be used for potentially lossy conversions between types
pub trait CheckedConv<T: ReprType, S: ReprType> {
    fn try_conv(
        itype: S,
        i: BinRep,
        otype: T,
    ) -> Result<BinRep, CheckedConvErr>;
}

pub fn msb(width: u32) -> u8 {
    let rem = width % 8;
    1u8 << (if rem != 0 { rem - 1 } else { 7 }) // shift to the right by between 0 and 7
}

/// attempt to sign extend the input data from in_width to out_width
/// this function assumes that you are 'okay' with treating the contents of the Untypednum as a signed number.
pub fn sign_extend_untyped(
    i: &BinRep,
    in_width: u32,
    out_width: u32,
) -> BinRep {
    let msb_idx = in_width.saturating_sub(1);
    let should_extend = (i) & (1 << msb_idx);
    if should_extend == 0 {
        return *i;
    }
    // otherwise, need to extend
    let pad_bits = out_width - in_width;
    if pad_bits > 64 {
        panic!(
            "sign extension error on input {:#x}: from {} to {}",
            i, in_width, out_width
        )
    }
    let mask_bits = ((1 << in_width) - 1) << msb_idx;
    mask_bits | *i
}
