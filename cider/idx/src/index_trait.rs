use smallvec::{SmallVec, smallvec};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// TODO(griffin): Replace with cranelift_entity if this ends up being the same
pub trait IndexRef: Copy + Eq {
    fn index(&self) -> usize;
    fn new(input: usize) -> Self;
}

/// A half open range of indices. The start is inclusive, the end is exclusive.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
        assert!(start <= end, "start must be less than or equal to end");
        Self { start, end }
    }

    /// Iterate over all the keys contained in the range
    pub fn iter(&self) -> IndexRangeIterator<'_, I> {
        IndexRangeIterator::new(self)
    }

    pub fn empty_interval() -> Self {
        Self {
            start: I::new(0),
            end: I::new(0),
        }
    }

    pub fn single_interval(item: I) -> Self {
        Self {
            start: item,
            end: I::new(item.index() + 1),
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

    pub fn nth_entry(&self, n: usize) -> I {
        assert!(n < self.size());
        I::new(self.start.index() + n)
    }
}

/// A continuous range of indices that is split into two parts.
///
/// Represents the ranges
/// `[start, split)` and `[split, end)`
#[derive(Debug, Clone)]
pub struct SplitIndexRange<I: IndexRef + PartialOrd> {
    start: I,
    split: I,
    end: I,
}

impl<I: IndexRef + PartialOrd> SplitIndexRange<I> {
    /// Create a new split index range.
    ///
    /// The `start` must be less than or equal to
    /// the `split`, and the `split` must be less than or equal to the `end`. It will
    /// panic if these conditions are not met.
    pub fn new(start: I, split: I, end: I) -> Self {
        assert!(start <= split);
        assert!(split <= end);

        Self { start, split, end }
    }

    /// Returns an iterator over the first segment of the range, i.e `[start, split)`.
    pub fn iter_first(&self) -> OwnedIndexRangeIterator<I> {
        OwnedIndexRangeIterator::new(IndexRange::new(self.start, self.split))
    }

    /// Returns an iterator over the second segment of the range, i.e `[split, end)`.
    pub fn iter_second(&self) -> OwnedIndexRangeIterator<I> {
        OwnedIndexRangeIterator::new(IndexRange::new(self.split, self.end))
    }

    /// Returns an iterator over the entire range.
    pub fn iter_all(&self) -> OwnedIndexRangeIterator<I> {
        OwnedIndexRangeIterator::new(IndexRange::new(self.start, self.end))
    }

    pub fn contains(&self, item: I) -> bool {
        item >= self.start && item < self.end
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

impl<I> ExactSizeIterator for IndexRangeIterator<'_, I> where
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

impl<I> Iterator for IndexRangeIterator<'_, I>
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
/// An iterator over a range of indices that owns the range, rather than borrowing it.
///
/// Because I really played myself by making the [IndexRangeIterator] have a
/// lifetime attached to it. This one doesn't do that. As with its sibling, the
/// range is half open, meaning that the start is inclusive, but the end is
/// exclusive.
pub struct OwnedIndexRangeIterator<I>
where
    I: IndexRef + PartialOrd,
{
    range: IndexRange<I>,
}

impl<I> OwnedIndexRangeIterator<I>
where
    I: IndexRef + PartialOrd,
{
    pub fn new(range: IndexRange<I>) -> Self {
        Self { range }
    }
}

impl<I> Iterator for OwnedIndexRangeIterator<I>
where
    I: IndexRef + PartialOrd,
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        if self.range.start < self.range.end {
            let out = self.range.start;
            self.range.start = I::new(self.range.start.index() + 1);
            Some(out)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = if self.range.end > self.range.start {
            self.range.end.index() - self.range.start.index()
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
                assert!(
                    last.end < value,
                    "Inserted value must not be less than end of the prior range"
                );
            }
        }
        self.0
            .push(IndexRange::new(value, I::new(value.index() + 1)));
    }

    pub fn iter(&self) -> impl Iterator<Item = I> {
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
