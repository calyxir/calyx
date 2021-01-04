use linked_hash_map::LinkedHashMap;
use std::ops::Index;

/// Attributes associated with a specific IR structure.
pub struct Attributes {
    /// Mapping from the name of the attribute to its value.
    attrs: LinkedHashMap<String, u64>,
}

impl Attributes {
    /// Add a new attribute
    pub fn insert<S>(&mut self, key: S, val: u64)
    where
        S: ToString + std::hash::Hash,
    {
        self.attrs.insert(key.to_string(), val);
    }

    /// Get the value associated with an attribute key
    pub fn get<S>(&self, key: &S) -> Option<&u64>
    where
        S: ToString + std::cmp::Eq,
    {
        self.attrs.get(&key.to_string())
    }

    /// Check if an attribute key has been set
    pub fn has<S>(&self, key: &S) -> bool
    where
        S: ToString + std::cmp::Eq,
    {
        self.attrs.contains_key(&key.to_string())
    }
}

impl<S> Index<&S> for Attributes
where
    S: ToString + std::cmp::Eq + std::fmt::Display,
{
    type Output = u64;

    fn index(&self, key: &S) -> &u64 {
        self.get(&key)
            .unwrap_or_else(|| panic!("No key `{}` in attribute map", key))
    }
}
