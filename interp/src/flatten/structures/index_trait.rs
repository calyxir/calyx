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
