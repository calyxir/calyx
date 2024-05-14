use super::index_trait::{IndexRange, IndexRangeIterator, IndexRef};
use std::{
    marker::PhantomData,
    ops::{self, Index},
};

#[derive(Debug)]
pub struct IndexedMap<K, D>
where
    K: IndexRef,
{
    data: Vec<D>,
    phantom: PhantomData<K>,
}

impl<K, D> IndexedMap<K, D>
where
    K: IndexRef + PartialOrd,
{
    /// Produces a range containing all the keys in the input map. This is
    /// similar to [IndexedMap::keys] but has an independent lifetime from the
    /// map
    pub fn range(&self) -> IndexRange<K> {
        IndexRange::new(K::new(0), K::new(self.len()))
    }
}

impl<K> IndexedMap<K, ()>
where
    K: IndexRef,
{
    /// Special case for empty tuple to enable a key generator.
    pub fn next_key(&mut self) -> K {
        self.push(())
    }
}

impl<K, D> ops::IndexMut<K> for IndexedMap<K, D>
where
    K: IndexRef,
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        &mut self.data[index.index()]
    }
}

impl<K, D> ops::Index<K> for IndexedMap<K, D>
where
    K: IndexRef,
{
    type Output = D;

    fn index(&self, index: K) -> &Self::Output {
        &self.data[index.index()]
    }
}

impl<K, D> IndexedMap<K, D>
where
    K: IndexRef,
{
    pub fn with_capacity(size: usize) -> Self {
        Self {
            data: Vec::with_capacity(size),
            phantom: PhantomData,
        }
    }

    pub fn new() -> Self {
        Self {
            data: Vec::new(),
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

    pub fn peek_next_idx(&self) -> K {
        K::new(self.data.len())
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &D)> {
        self.data.iter().enumerate().map(|(i, v)| (K::new(i), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (K, &mut D)> {
        self.data
            .iter_mut()
            .enumerate()
            .map(|(i, v)| (K::new(i), v))
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut D> {
        self.data.iter_mut()
    }

    pub fn keys(&self) -> impl Iterator<Item = K> + '_ {
        // TODO (griffin): Make this an actual struct instead
        self.data.iter().enumerate().map(|(i, _)| K::new(i))
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    pub fn first(&self) -> Option<&D> {
        self.data.first()
    }
}

impl<T, K> Default for IndexedMap<K, T>
where
    K: IndexRef,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct IndexedMapRangeIterator<'range, 'data, K, D>
where
    K: IndexRef + PartialOrd,
{
    iterator: IndexRangeIterator<'range, K>,
    data: &'data IndexedMap<K, D>,
}

impl<'range, 'data, K, D> ExactSizeIterator
    for IndexedMapRangeIterator<'range, 'data, K, D>
where
    K: IndexRef + PartialOrd,
{
}

impl<'range, 'data, K, D> Iterator
    for IndexedMapRangeIterator<'range, 'data, K, D>
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
pub struct AuxillaryMap<K, D>
where
    K: IndexRef,
    D: Clone,
{
    data: Vec<D>,
    phantom: PhantomData<K>,
    default_value: D,
}

// NOTE TO SELF: do not implement IndexMut

impl<K, D> Index<K> for AuxillaryMap<K, D>
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

impl<K, D> AuxillaryMap<K, D>
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
            data: Vec::with_capacity(size),
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

impl<K, D> AuxillaryMap<K, D>
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
            data: Vec::with_capacity(size),
            phantom: PhantomData,
            default_value: Default::default(),
        }
    }
}

impl<K, D> Default for AuxillaryMap<K, D>
where
    K: IndexRef,
    D: Clone + Default,
{
    fn default() -> Self {
        Self::new()
    }
}
