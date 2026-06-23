// format-independent representation

use super::numrep as nr;

// TODO: distinguish binary formats and text formats, s.t. to_data can be either a string iterator or a bits iterator

pub trait BinFileFmt<T: nr::ReprType> {
    fn to_data(self) -> nr::DataSet<T>;

    // if data formats wish to trim to a precise number of bits, they may do so on the BinRep

    fn from_bin(t: impl Iterator<Item = nr::BinRep>, width: u32) -> Self;
}

// string formats inherently (possibly) introduce loss at the conversion stage
pub trait StringFormat<T: nr::ReprType> {}

// corresponding traits for lossy binary / lossless strings are left unimplemented, they're not used for now

/// interface for data formats
pub trait DataFormat<T: nr::ReprType> {
    fn to_data(self) -> nr::DataSet<T>;
    fn from_bin(t: impl Iterator<Item = nr::BinRep>, width: u32) -> Self;
}

// these should probably not be here...
pub struct Untyped<const WIDTH: u32>;

pub struct Int<const WIDTH: u32>;

// these generics can be used to implement the 32/64-bit variants of fixed and float
struct WrappedFixed<const WIDTH: u32>;
struct WrappedFloat<const WIDTH: u32>;

// pub type Fixed32 = WrappedFixed<32>;
// pub type Fixed64 = WrappedFixed<64>;
// pub type Float32 = WrappedFloat<32>;
// pub type Float64 = WrappedFloat<64>;
