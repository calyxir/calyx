// TODO(griffin): Replace with cranelift_entity if this ends up being the same
pub trait IndexRef: Copy + Eq {
    fn index(&self) -> usize;
    fn new(input: usize) -> Self;
}

/// This macro is used to implement the IndexRef trait for a type that wraps an
/// unsigned integer value. By default, the macro will implement the trait using
/// a [`u32`](std::u32) as the backing type. However, if a different backing type
/// is desired, it can be specified as the second argument.
macro_rules! impl_index {
    ($struct_name: ident) => {
        impl_index!($struct_name, u32);
    };

    ($struct_name: ident, $backing_ty: ty) => {
        impl $crate::flatten::structures::index_trait::IndexRef
            for $struct_name
        {
            fn index(&self) -> usize {
                self.0 as usize
            }

            fn new(input: usize) -> Self {
                Self(input as $backing_ty)
            }
        }

        impl From<$backing_ty> for $struct_name {
            fn from(input: $backing_ty) -> Self {
                $struct_name(input)
            }
        }

        impl From<usize> for $struct_name {
            fn from(input: usize) -> Self {
                $crate::flatten::structures::index_trait::IndexRef::new(input)
            }
        }
    };
}
/// This macro is used to implement the IndexRef trait for a type that wraps a
/// NonZero value. By default, the macro will implement the trait using a
/// [`NonZeroU32`](std::num::NonZeroU32) as the backing type. However, if a
/// different backing type is desired, it can be specified as the second
/// argument to the macro.
macro_rules! impl_index_nonzero {
    // Cool and normal stuff here
    ($struct_name: ident) => {
        impl_index_nonzero!($struct_name, std::num::NonZeroU32, u32);
    };

    ($struct_name: ident, NonZeroU8) => {
        impl_index_nonzero!($struct_name, std::num::NonZeroU8, u8);
    };

    ($struct_name: ident, NonZeroU16) => {
        impl_index_nonzero!($struct_name, std::num::NonZeroU16, u16);
    };

    ($struct_name: ident, NonZeroU32) => {
        impl_index_nonzero!($struct_name, std::num::NonZeroU32, u32);
    };

    ($struct_name: ident, $non_zero_type:ty, $normal_type:ty) => {
        impl $crate::flatten::structures::index_trait::IndexRef
            for $struct_name
        {
            fn index(&self) -> usize {
                self.0.get() as usize - 1
            }

            fn new(input: usize) -> Self {
                Self(
                    <$non_zero_type>::new((input + 1) as $normal_type).unwrap(),
                )
            }
        }

        impl From<$non_zero_type> for $struct_name {
            fn from(input: $non_zero_type) -> Self {
                $struct_name(input)
            }
        }

        impl From<usize> for $struct_name {
            fn from(input: usize) -> Self {
                $crate::flatten::structures::index_trait::IndexRef::new(input)
            }
        }
    };
}

use smallvec::{smallvec, SmallVec};

use crate::flatten::flat_ir::base::LocalPortOffset;

pub(crate) use {impl_index, impl_index_nonzero};

/// A half open range of indices. The start is inclusive, the end is exclusive.
#[derive(Debug, Clone, Copy)]
pub struct IndexRange<I>
where
    I: IndexRef + PartialOrd,
{
    /// The start of the range (inclusive).
    start: I,
    /// The end of the range (exclusive).
    end: I,
}

impl<I> From<(I, I)> for IndexRange<I>
where
    I: IndexRef + PartialOrd,
{
    fn from(value: (I, I)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl<I> IndexRange<I>
where
    I: IndexRef + PartialOrd,
{
    pub fn new(start: I, end: I) -> Self {
        debug_assert!(start <= end, "start must be less than or equal to end");
        Self { start, end }
    }

    /// Iterate over all the keys contained in the range
    pub fn iter(&self) -> IndexRangeIterator<I> {
        IndexRangeIterator::new(self)
    }

    pub fn empty_interval() -> Self {
        Self {
            start: I::new(0),
            end: I::new(0),
        }
    }

    pub fn size(&self) -> usize {
        self.end.index().saturating_sub(self.start.index())
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn contains(&self, candidate: I) -> bool {
        self.start <= candidate && self.end > candidate
    }

    pub fn set_end(&mut self, end: I) {
        self.end = end;
    }

    pub fn start(&self) -> I {
        self.start
    }

    pub fn end(&self) -> I {
        self.end
    }
}

impl<I> IntoIterator for IndexRange<I>
where
    I: IndexRef + PartialOrd,
{
    type Item = I;

    type IntoIter = OwnedIndexRangeIterator<I>;

    fn into_iter(self) -> Self::IntoIter {
        OwnedIndexRangeIterator::new(self)
    }
}

impl<'a, I> IntoIterator for &'a IndexRange<I>
where
    I: IndexRef + PartialOrd,
{
    type Item = I;

    type IntoIter = IndexRangeIterator<'a, I>;

    fn into_iter(self) -> Self::IntoIter {
        IndexRangeIterator::new(self)
    }
}

#[derive(Debug)]
pub struct IndexRangeIterator<'a, I>
where
    I: IndexRef + PartialOrd,
{
    range: &'a IndexRange<I>,
    current: I,
}

impl<'a, I> ExactSizeIterator for IndexRangeIterator<'a, I> where
    I: IndexRef + PartialOrd
{
}

impl<'a, I> IndexRangeIterator<'a, I>
where
    I: IndexRef + PartialOrd,
{
    pub fn new(range: &'a IndexRange<I>) -> Self {
        Self {
            range,
            current: range.start,
        }
    }
}

impl<'a, I> Iterator for IndexRangeIterator<'a, I>
where
    I: IndexRef + PartialOrd,
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.range.end {
            let current = self.current;
            self.current = I::new(self.current.index() + 1);
            Some(current)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = if self.range.end.index() > self.current.index() {
            self.range.end.index() - self.current.index()
        } else {
            0
        };

        (size, Some(size))
    }
}

/// Because I really played myself by making the [IndexRangeIterator] have a
/// lifetime attached to it. This one doesn't do that. As with it's sibling, the
/// range is half open, meaning that the start is inclusive, but the end is
/// exclusive.
pub struct OwnedIndexRangeIterator<I>
where
    I: IndexRef + PartialOrd,
{
    range: IndexRange<I>,
    current: I,
}

impl<I> OwnedIndexRangeIterator<I>
where
    I: IndexRef + PartialOrd,
{
    pub fn new(range: IndexRange<I>) -> Self {
        Self {
            range,
            current: range.start,
        }
    }
}

impl<I> Iterator for OwnedIndexRangeIterator<I>
where
    I: IndexRef + PartialOrd,
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.range.end {
            let current = self.current;
            self.current = I::new(self.current.index() + 1);
            Some(current)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = if self.range.end.index() > self.current.index() {
            self.range.end.index() - self.current.index()
        } else {
            0
        };

        (size, Some(size))
    }
}

impl<I> ExactSizeIterator for OwnedIndexRangeIterator<I> where
    I: IndexRef + PartialOrd
{
}

#[derive(Debug, Clone)]
pub struct ConcatenatedIndexRanges<I, const N: usize>(
    SmallVec<[IndexRange<I>; N]>,
)
where
    I: IndexRef + PartialOrd;

impl<I, const N: usize> std::ops::Add<IndexRange<I>>
    for ConcatenatedIndexRanges<I, N>
where
    I: IndexRef + PartialOrd,
{
    type Output = Self;

    fn add(mut self, rhs: IndexRange<I>) -> Self::Output {
        self.append(rhs);
        self
    }
}

impl<I, const N: usize> std::ops::AddAssign<IndexRange<I>>
    for ConcatenatedIndexRanges<I, N>
where
    I: IndexRef + PartialOrd,
{
    fn add_assign(&mut self, rhs: IndexRange<I>) {
        self.append(rhs);
    }
}

impl<I, const N: usize> std::ops::AddAssign<I> for ConcatenatedIndexRanges<I, N>
where
    I: IndexRef + PartialOrd,
{
    fn add_assign(&mut self, rhs: I) {
        self.append_item(rhs);
    }
}

impl<I, const N: usize> ConcatenatedIndexRanges<I, N>
where
    I: IndexRef + PartialOrd,
{
    pub fn new() -> Self {
        Self(SmallVec::new())
    }

    /// Appends a range to the concatenated ranges. This requires that the
    /// appended range starts after the last range in the concatenation. If it
    /// is consecutive with the last range, then the last range will be extended.
    pub fn append(&mut self, value: IndexRange<I>) {
        if let Some(last) = self.0.last_mut() {
            if last.end == value.start {
                last.set_end(value.end);
                return;
            } else {
                assert!(
                    last.end < value.start,
                    "Ranges must be strictly increasing"
                );
            }
        }

        self.0.push(value);
    }

    pub fn append_item(&mut self, value: I) {
        if let Some(last) = self.0.last_mut() {
            if last.end == value {
                last.set_end(I::new(last.end.index() + 1));
                return;
            } else {
                assert!(last.end < value, "Inserted value must not be less than end of the prior range");
            }
        }
        self.0
            .push(IndexRange::new(value, I::new(value.index() + 1)));
    }

    pub fn iter(&self) -> impl Iterator<Item = I> + '_ {
        self.0.iter().flat_map(|x| x.iter())
    }

    pub fn contains(&self, candidate: I) -> bool {
        for range in self.0.iter() {
            if range.start > candidate {
                return false;
            } else if range.end > candidate {
                return true;
            } else {
                continue;
            }
        }
        false
    }

    pub fn first(&self) -> Option<I> {
        self.0.first().map(|x| x.start)
    }

    pub fn last(&self) -> Option<I> {
        self.0.last().map(|x| x.end)
    }
}

impl<I, const N: usize> Default for ConcatenatedIndexRanges<I, N>
where
    I: IndexRef + PartialOrd,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I, const N: usize> From<IndexRange<I>> for ConcatenatedIndexRanges<I, N>
where
    I: IndexRef + PartialOrd,
{
    fn from(value: IndexRange<I>) -> Self {
        Self(smallvec![value])
    }
}

pub type SignatureRange = ConcatenatedIndexRanges<LocalPortOffset, 2>;
