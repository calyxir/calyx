use std::{hash::Hash, ops::Index};

use super::index_trait::IndexRef;
use ahash::{HashMap, HashMapExt};

#[derive(Debug, Default)]
pub struct SparseMap<K, D>
where
    K: IndexRef + Hash,
{
    data: HashMap<K, D>,
    count: usize,
}

impl<K, D> Index<K> for SparseMap<K, D>
where
    K: IndexRef + Hash,
{
    type Output = D;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index).expect("SparseMap index out of bounds.")
    }
}

impl<K, D> SparseMap<K, D>
where
    K: IndexRef + Hash,
{
    pub fn new() -> Self {
        Self {
            data: Default::default(),
            count: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: HashMap::with_capacity(capacity),
            count: 0,
        }
    }

    pub fn insert(&mut self, value: D) -> K {
        let index = K::new(self.count);
        self.data.insert(index, value);
        self.count += 1;
        index
    }

    pub fn get(&self, index: K) -> Option<&D> {
        self.data.get(&index)
    }

    pub fn get_mut(&mut self, index: K) -> Option<&mut D> {
        self.data.get_mut(&index)
    }

    /// Skips the next `skip_count` indices. Used to advance the index counter.
    pub fn skip(&mut self, skip_count: usize) {
        self.count += skip_count;
    }

    pub fn peek_next_index(&self) -> K {
        K::new(self.count)
    }
}
