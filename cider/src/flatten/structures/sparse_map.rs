use std::{hash::Hash, ops::Index};

use super::index_trait::{IndexRange, IndexRef};
use ahash::{HashMap, HashMapExt};

#[derive(Debug, Clone)]
pub struct SparseMap<K, D>
where
    K: IndexRef + Hash + PartialOrd,
{
    data: HashMap<K, D>,
    /// An internal list of ranges that are used to iterate over the map in
    /// insertion order. A bit of cleverness which allows us to avoid storing
    /// every index
    iteration_order: Vec<IndexRange<K>>,
    count: usize,
}

// This is not quite the same as the derived version, sorry!
impl<K, D> Default for SparseMap<K, D>
where
    K: IndexRef + Hash + PartialOrd,
{
    fn default() -> Self {
        Self {
            data: HashMap::new(),
            iteration_order: vec![IndexRange::empty_interval()],
            count: 0,
        }
    }
}

impl<K, D> Index<K> for SparseMap<K, D>
where
    K: IndexRef + Hash + PartialOrd,
{
    type Output = D;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index).expect("SparseMap index out of bounds.")
    }
}

impl<K, D> SparseMap<K, D>
where
    K: IndexRef + Hash + PartialOrd,
{
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            iteration_order: vec![IndexRange::empty_interval()],
            count: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: HashMap::with_capacity(capacity),
            iteration_order: vec![IndexRange::empty_interval()],
            count: 0,
        }
    }

    pub fn insert(&mut self, value: D) -> K {
        let index = K::new(self.count);
        self.data.insert(index, value);
        self.count += 1;

        self.iteration_order
            .last_mut()
            .unwrap()
            .set_end(K::new(self.count));
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
        self.iteration_order
            .push(IndexRange::new(K::new(self.count), K::new(self.count)));
    }

    pub fn peek_next_index(&self) -> K {
        K::new(self.count)
    }

    /// The count index value
    pub fn count(&self) -> usize {
        self.count
    }

    /// Iterates over all the keys and values in the map in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (K, &D)> {
        self.keys().map(|x| (x, self.get(x).unwrap()))
    }

    /// Iterates over all the keys in the map in insertion order.
    /// This returns `K` rather than `&K` because it internally uses
    /// [IndexRange]s
    pub fn keys(&self) -> impl Iterator<Item = K> + '_ {
        self.iteration_order.iter().flat_map(|x| x.iter())
    }

    pub fn values(&self) -> impl Iterator<Item = &D> {
        self.data.values()
    }

    pub fn contains(&self, index: K) -> bool {
        self.data.contains_key(&index)
    }
}

/// An analogue to [AuxiliaryMap](super::indexed_map::AuxiliaryMap) for sparse
/// maps. This is used to store extra information that is only applicable to a
/// subset of the indices in a primary map.
#[derive(Debug, Clone)]
pub struct AuxiliarySparseMap<K, D>
where
    K: IndexRef + Hash,
{
    data: HashMap<K, D>,
}

impl<K, D> AuxiliarySparseMap<K, D>
where
    K: IndexRef + Hash,
{
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert_value(&mut self, key: K, value: D) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: K) -> Option<&D> {
        self.data.get(&key)
    }
}

impl<K, D> Default for AuxiliarySparseMap<K, D>
where
    K: IndexRef + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}
