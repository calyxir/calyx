// TODO(griffin): Replace with cranelift_entity if this ends up being the same
pub trait IndexRef: Copy + Eq {
    fn index(&self) -> usize;
    fn new(input: usize) -> Self;
}

macro_rules! impl_index {
    ($v: vis $struct_name: ident) => {
        impl_index!($v $struct_name, u32);
    };

    ( $v:vis $struct_name: ident, $backing_ty: ty) => {
        #[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
        $v struct $struct_name($backing_ty);

        impl $crate::flatten::structures::index_trait::IndexRef for $struct_name {
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
    };
}

pub(crate) use impl_index;

#[derive(Debug)]
pub struct IndexRange<I>
where
    I: IndexRef + PartialOrd,
{
    /// The start of the range (inclusive).
    start: I,
    /// The end of the range (inclusive).
    end: I,
}

impl<I> IndexRange<I>
where
    I: IndexRef + PartialOrd,
{
    pub fn new(start: I, end: I) -> Self {
        Self { start, end }
    }

    pub fn iter(&self) -> IndexRangeIterator<I> {
        IndexRangeIterator::new(self)
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
        if self.current <= self.range.end {
            let current = self.current;
            self.current = I::new(self.current.index() + 1);
            Some(current)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = if self.range.end.index() >= self.current.index() {
            self.range.end.index() - self.current.index() + 1
        } else {
            0
        };

        (size, Some(size))
    }
}
