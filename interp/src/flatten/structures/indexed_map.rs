use smallvec::SmallVec;

use super::index_trait::{IndexRangeIterator, IndexRef};
use std::{marker::PhantomData, ops};

#[derive(Debug)]
pub struct IndexedMap<D, K, const N: usize = 0>
where
    K: IndexRef,
{
    data: SmallVec<[D; N]>,
    phantom: PhantomData<K>,
}

impl<D, K, const N: usize> ops::IndexMut<K> for IndexedMap<D, K, N>
where
    K: IndexRef,
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        &mut self.data[index.index()]
    }
}

impl<D, K, const N: usize> ops::Index<K> for IndexedMap<D, K, N>
where
    K: IndexRef,
{
    type Output = D;

    fn index(&self, index: K) -> &Self::Output {
        &self.data[index.index()]
    }
}

impl<D, K, const N: usize> IndexedMap<D, K, N>
where
    K: IndexRef,
{
    pub fn with_capacity(size: usize) -> Self {
        Self {
            data: SmallVec::with_capacity(size),
            phantom: PhantomData,
        }
    }

    pub fn new() -> Self {
        Self {
            data: SmallVec::new(),
            phantom: PhantomData,
        }
    }

    pub fn get(&self, index: K) -> Option<&D> {
        if index.index() < self.data.len() {
            Some(&self.data[index.index()])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: K) -> Option<&mut D> {
        if index.index() < self.data.len() {
            Some(&mut self.data[index.index()])
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn push(&mut self, item: D) -> K {
        self.data.push(item);
        K::new(self.data.len() - 1)
    }

    pub fn next_idx(&self) -> K {
        K::new(self.data.len())
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T, K, const N: usize> Default for IndexedMap<T, K, N>
where
    K: IndexRef,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct IndexedMapRangeIterator<'range, 'data, D, K, const N: usize>
where
    K: IndexRef + PartialOrd,
{
    iterator: IndexRangeIterator<'range, K>,
    data: &'data IndexedMap<D, K, N>,
}

impl<'range, 'data, D, K, const N: usize> ExactSizeIterator
    for IndexedMapRangeIterator<'range, 'data, D, K, N>
where
    K: IndexRef + PartialOrd,
{
}

impl<'range, 'data, D, K, const N: usize> Iterator
    for IndexedMapRangeIterator<'range, 'data, D, K, N>
where
    K: IndexRef + PartialOrd,
{
    type Item = &'data D;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(idx) = self.iterator.next() {
            Some(&self.data[idx])
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iterator.size_hint()
    }
}

#[derive(Debug)]
pub struct AuxillaryMap<D, K, const N: usize = 0>
where
    K: IndexRef,
    D: Clone,
{
    data: SmallVec<[D; N]>,
    phantom: PhantomData<K>,
    default_value: D,
}

impl<D, K, const N: usize> AuxillaryMap<D, K, N>
where
    K: IndexRef,
    D: Clone,
{
}

impl<D, K, const N: usize> AuxillaryMap<D, K, N>
where
    K: IndexRef,
    D: Clone + Default,
{
    pub fn new() -> Self {
        Self {
            data: Default::default(),
            phantom: PhantomData,
            default_value: Default::default(),
        }
    }
}
