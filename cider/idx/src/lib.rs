mod index_trait;
mod indexed_map;
mod macros;
mod sparse_map;

pub mod maps {
    pub use super::indexed_map::{IndexedMap, SecondaryMap};
    pub use super::sparse_map::{SecondarySparseMap, SparseMap};
}

pub mod iter {
    pub use super::index_trait::{
        ConcatenatedIndexRanges, IndexRange, IndexRangeIterator,
        OwnedIndexRangeIterator, SplitIndexRange,
    };
    pub use super::indexed_map::IndexedMapRangeIterator;
}

pub use index_trait::IndexRef;
