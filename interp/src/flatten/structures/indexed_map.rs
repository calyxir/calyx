use super::index_trait::IndexRef;
use std::{marker::PhantomData, ops};

pub struct IndexedMap<T, K>
where
    K: IndexRef,
{
    data: Vec<T>,
    phantom: PhantomData<K>,
}

impl<T, K> ops::IndexMut<K> for IndexedMap<T, K>
where
    K: IndexRef,
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        &mut self.data[index.index()]
    }
}

impl<T, K> ops::Index<K> for IndexedMap<T, K>
where
    K: IndexRef,
{
    type Output = T;

    fn index(&self, index: K) -> &Self::Output {
        &self.data[index.index()]
    }
}

impl<T, K> IndexedMap<T, K>
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

    pub fn get(&self, index: K) -> Option<&T> {
        if index.index() < self.data.len() {
            Some(&self.data[index.index()])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: K) -> Option<&mut T> {
        if index.index() < self.data.len() {
            Some(&mut self.data[index.index()])
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn push(&mut self, item: T) -> K {
        self.data.push(item);
        K::new(self.data.len() - 1)
    }
}
