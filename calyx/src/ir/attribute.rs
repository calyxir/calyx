use linked_hash_map::LinkedHashMap;
use std::ops::Index;

/// Attributes associated with a specific IR structure.
#[derive(Debug, Clone)]
pub struct Attributes {
    /// Mapping from the name of the attribute to its value.
    pub(super) attrs: LinkedHashMap<String, u64>,
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            // Does not allocate any space.
            attrs: LinkedHashMap::with_capacity(0),
        }
    }
}

impl From<Vec<(String, u64)>> for Attributes {
    fn from(v: Vec<(String, u64)>) -> Self {
        Attributes {
            attrs: v.into_iter().collect(),
        }
    }
}

/// Structs that can return an [`Attributes`] instance.
pub trait GetAttributes {
    /// Returns an [`Attributes`] instance
    fn get_attributes(&self) -> Option<&Attributes>;

    /// Returns a mutable [`Attributes`] instance
    fn get_mut_attributes(&mut self) -> Option<&mut Attributes>;
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
    pub fn get<S>(&self, key: S) -> Option<&u64>
    where
        S: std::fmt::Display + AsRef<str>,
    {
        self.attrs.get(&key.as_ref().to_string())
    }

    /// Check if an attribute key has been set
    pub fn has<S>(&self, key: S) -> bool
    where
        S: std::fmt::Display + AsRef<str>,
    {
        self.attrs.contains_key(&key.as_ref().to_string())
    }

    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty()
    }
}

impl<S> Index<&S> for Attributes
where
    S: AsRef<str> + std::fmt::Display,
{
    type Output = u64;

    fn index(&self, key: &S) -> &u64 {
        self.get(&key)
            .unwrap_or_else(|| panic!("No key `{}` in attribute map", key))
    }
}
