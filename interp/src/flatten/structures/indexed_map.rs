use smallvec::SmallVec;

use super::index_trait::{IndexRangeIterator, IndexRef};
use std::{
    marker::PhantomData,
    ops::{self, Index},
};

#[derive(Debug)]
pub struct IndexedMap<K, D, const N: usize = 0>
where
    K: IndexRef,
{
    data: SmallVec<[D; N]>,
    phantom: PhantomData<K>,
}

impl<K, D, const N: usize> ops::IndexMut<K> for IndexedMap<K, D, N>
where
    K: IndexRef,
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        &mut self.data[index.index()]
    }
}

impl<K, D, const N: usize> ops::Index<K> for IndexedMap<K, D, N>
where
    K: IndexRef,
{
    type Output = D;

    fn index(&self, index: K) -> &Self::Output {
        &self.data[index.index()]
    }
}

impl<K, D, const N: usize> IndexedMap<K, D, N>
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

impl<T, K, const N: usize> Default for IndexedMap<K, T, N>
where
    K: IndexRef,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct IndexedMapRangeIterator<'range, 'data, K, D, const N: usize>
where
    K: IndexRef + PartialOrd,
{
    iterator: IndexRangeIterator<'range, K>,
    data: &'data IndexedMap<K, D, N>,
}

impl<'range, 'data, K, D, const N: usize> ExactSizeIterator
    for IndexedMapRangeIterator<'range, 'data, K, D, N>
where
    K: IndexRef + PartialOrd,
{
}

impl<'range, 'data, K, D, const N: usize> Iterator
    for IndexedMapRangeIterator<'range, 'data, K, D, N>
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
pub struct AuxillaryMap<K, D, const N: usize = 0>
where
    K: IndexRef,
    D: Clone,
{
    data: SmallVec<[D; N]>,
    phantom: PhantomData<K>,
    default_value: D,
}

// NOTE TO SELF: do not implement IndexMut

impl<K, D, const N: usize> Index<K> for AuxillaryMap<K, D, N>
where
    K: IndexRef,
    D: Clone,
{
    type Output = D;

    fn index(&self, index: K) -> &Self::Output {
        if index.index() < self.data.len() {
            &self.data[index.index()]
        } else {
            &self.default_value
        }
    }
}

impl<K, D, const N: usize> AuxillaryMap<K, D, N>
where
    K: IndexRef,
    D: Clone,
{
    pub fn new_with_default(default_value: D) -> Self {
        Self {
            data: Default::default(),
            phantom: PhantomData,
            default_value,
        }
    }

    pub fn capacity_with_default(default_value: D, size: usize) -> Self {
        Self {
            data: SmallVec::with_capacity(size),
            phantom: PhantomData,
            default_value,
        }
    }

    pub fn get(&self, index: K) -> &D {
        if index.index() < self.data.len() {
            &self.data[index.index()]
        } else {
            &self.default_value
        }
    }

    pub fn push(&mut self, item: D) {
        self.data.push(item);
    }

    pub fn insert(&mut self, index: K, item: D) {
        if index.index() < self.data.len() {
            self.data[index.index()] = item;
        } else {
            self.data
                .resize(index.index() + 1, self.default_value.clone());
            self.data[index.index()] = item;
        }
    }
}

impl<K, D, const N: usize> AuxillaryMap<K, D, N>
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

    pub fn with_capacity(size: usize) -> Self {
        Self {
            data: SmallVec::with_capacity(size),
            phantom: PhantomData,
            default_value: Default::default(),
        }
    }
}

impl<K, D, const N: usize> Default for AuxillaryMap<K, D, N>
where
    K: IndexRef,
    D: Clone + Default,
{
    fn default() -> Self {
        Self::new()
    }
}
