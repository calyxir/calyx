use super::index_trait::{IndexRange, IndexRangeIterator, IndexRef};
use std::{
    marker::PhantomData,
    ops::{self, Index},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IndexedMap<K, D>
where
    K: IndexRef,
{
    data: Vec<D>,
    phantom: PhantomData<K>,
}

impl<K, D> IndexedMap<K, D>
where
    K: IndexRef + Ord + Eq,
{
    /// Returns two mutable accesses to the given indices in the order they were
    /// given. The indices must be valid and must not be the same index
    /// otherwise this function will return None
    pub fn split_mut_indices(
        &mut self,
        idx1: K,
        idx2: K,
    ) -> Option<(&mut D, &mut D)> {
        if idx1 == idx2
            || idx1.index() >= self.data.len()
            || idx2.index() >= self.data.len()
        {
            None
        } else if idx1 < idx2 {
            let split_point = idx1.index() + 1;
            let (slice_1, slice_2) = self.data.split_at_mut(split_point);
            Some((
                slice_1.last_mut().unwrap(),
                &mut slice_2[idx2.index() - split_point],
            ))
        }
        // idx2 is smaller
        else {
            let split_point = idx2.index() + 1;
            let (slice_2, slice_1) = self.data.split_at_mut(split_point);
            Some((
                &mut slice_1[idx1.index() - split_point],
                slice_2.last_mut().unwrap(),
            ))
        }
    }
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

    pub fn values(&self) -> impl Iterator<Item = &D> {
        self.data.iter()
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

#[allow(dead_code)]
pub struct IndexedMapRangeIterator<'range, 'data, K, D>
where
    K: IndexRef + PartialOrd,
{
    iterator: IndexRangeIterator<'range, K>,
    data: &'data IndexedMap<K, D>,
}

impl<K, D> ExactSizeIterator for IndexedMapRangeIterator<'_, '_, K, D> where
    K: IndexRef + PartialOrd
{
}

impl<'data, K, D> Iterator for IndexedMapRangeIterator<'_, 'data, K, D>
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SecondaryMap<K, D>
where
    K: IndexRef,
    D: Clone,
{
    data: Vec<D>,
    phantom: PhantomData<K>,
    default_value: D,
}

// NOTE TO SELF: do not implement IndexMut

impl<K, D> Index<K> for SecondaryMap<K, D>
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

impl<K, D> SecondaryMap<K, D>
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

    pub fn iter(&self) -> impl Iterator<Item = (K, &D)> {
        self.data.iter().enumerate().map(|(k, v)| (K::new(k), v))
    }
}

impl<K, D> SecondaryMap<K, D>
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

impl<K, D> Default for SecondaryMap<K, D>
where
    K: IndexRef,
    D: Clone + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{impl_index, maps::IndexedMap};

    #[test]
    fn test_split_mut() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        struct MyIdx(u32);
        impl_index!(MyIdx);

        #[derive(Debug, Clone, PartialEq, Eq)]
        struct MyData {
            number: usize,
        }

        let mut data_map: IndexedMap<MyIdx, MyData> = IndexedMap::new();

        for i in 0_usize..4000 {
            data_map.push(MyData { number: i });
        }

        let first_idx = MyIdx::from(1234_usize);
        let second_idx = MyIdx::from(3210_usize);

        let (first_mut, second_mut) =
            data_map.split_mut_indices(first_idx, second_idx).unwrap();

        assert_eq!(first_mut.number, 1234);
        assert_eq!(second_mut.number, 3210);

        let raw_1 = first_mut as *mut MyData;
        let raw_2 = second_mut as *mut MyData;

        first_mut.number = 7001;
        second_mut.number = 7002;
        let (second_mut, first_mut) =
            data_map.split_mut_indices(second_idx, first_idx).unwrap();

        assert_eq!(raw_1, first_mut as *mut MyData);
        assert_eq!(raw_2, second_mut as *mut MyData);

        assert_eq!(first_mut.number, 7001);
        assert_eq!(second_mut.number, 7002);
    }
}
