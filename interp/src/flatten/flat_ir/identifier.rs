use std::collections::HashMap;
use std::hash::Hash;

use crate::flatten::structures::index_trait::impl_index;

impl_index!(pub Identifier);

impl Identifier {
    #[inline]
    pub fn get_default_id() -> Identifier {
        // manually construct
        Identifier(0)
    }
}
#[derive(Debug)]
pub struct IdMap {
    count: u32,
    forward: HashMap<String, Identifier>,
    backward: HashMap<Identifier, String>,
}

impl IdMap {
    /// inner builder style utility function
    fn insert_empty_string(mut self) -> Self {
        self.insert("");
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
            forward: HashMap::with_capacity(capacity + 1),
            backward: HashMap::with_capacity(capacity + 1),
        }
        .insert_empty_string()
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
    pub fn lookup_id(&self, key: &String) -> Option<&Identifier> {
        self.forward.get(key)
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
