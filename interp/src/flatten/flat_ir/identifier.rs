use std::collections::HashMap;
use std::hash::Hash;

use crate::flatten::structures::index_trait::impl_index;

impl_index!(pub Identifier);

#[derive(Debug, Default)]
pub struct IdMap {
    count: u32,
    forward: HashMap<String, Identifier>,
    backward: HashMap<Identifier, String>,
}

impl IdMap {
    pub fn new() -> Self {
        Self::default()
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

    pub fn lookup_id(&self, key: &String) -> Option<&Identifier> {
        self.forward.get(key)
    }

    pub fn lookup_string(&self, id: &Identifier) -> Option<&String> {
        self.backward.get(id)
    }
}
