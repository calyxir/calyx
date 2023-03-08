use ahash::{HashMap, HashMapExt};
use std::hash::Hash;

use crate::flatten::structures::index_trait::impl_index;

impl_index!(pub Identifier);

impl Identifier {
    #[inline]
    pub(crate) fn get_default_id() -> Identifier {
        // manually construct
        Identifier(0)
    }
}

/// This is using the [ahash] crate instead of the std
/// [HashMap](std::collections::HashMap) for general speed though that is likely
/// unnecessary as this should not be on any hot paths. If we want to be
/// resistant to hash attacks the forward map can be changed to be amenable to
/// that though we're generating with randomness so that is unlikely to be an
/// issue
#[derive(Debug)]
pub struct IdMap {
    count: u32,
    forward: HashMap<String, Identifier>,
    backward: HashMap<Identifier, String>,
}

impl IdMap {
    /// number of strings that are included by default. Used when constructing a
    /// table with a specific capacity
    const PREALLOCATED: usize = 3;

    /// inner builder style utility function
    fn insert_basic_strings(mut self) -> Self {
        self.insert("");
        self.insert("go");
        self.insert("done");
        self
    }

    /// Initializes a new identifier map with the empty string pre-inserted
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Initializes a new identifier map with the given number of slots
    /// preallocated and the empty string inserted
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            count: 0,
            forward: HashMap::with_capacity(capacity + Self::PREALLOCATED),
            backward: HashMap::with_capacity(capacity + Self::PREALLOCATED),
        }
        .insert_basic_strings()
    }

    /// Inserts a string mapping into the table and returns the identifier.
    /// If already present, the original identifier is returned
    pub fn insert<S>(&mut self, input: S) -> Identifier
    where
        S: AsRef<str>,
    {
        let id = self
            .forward
            .entry(input.as_ref().to_string())
            .or_insert_with_key(|k| {
                let id = Identifier::from(self.count);
                self.count += 1;

                self.backward.insert(id, k.clone());
                id
            });

        *id
    }

    /// Returns the identifier associated with the string, if present
    pub fn lookup_id<S: AsRef<str>>(&self, key: S) -> Option<&Identifier> {
        self.forward.get(key.as_ref())
    }

    /// Returns the string associated with the identifier, if present
    pub fn lookup_string(&self, id: &Identifier) -> Option<&String> {
        self.backward.get(id)
    }
}

impl Default for IdMap {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
