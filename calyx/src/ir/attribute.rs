use linked_hash_map::LinkedHashMap;
use std::{convert::TryFrom, ops::Index};

use crate::errors::{CalyxResult, Span, WithPos};

/// Attributes associated with a specific IR structure.
#[derive(Debug, Clone)]
pub struct Attributes {
    /// Mapping from the name of the attribute to its value.
    pub(super) attrs: LinkedHashMap<String, u64>,
    /// Source location information for the item
    span: Option<Span>,
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            // Does not allocate any space.
            attrs: LinkedHashMap::with_capacity(0),
            span: None,
        }
    }
}

impl TryFrom<Vec<(String, u64)>> for Attributes {
    type Error = crate::errors::Error;

    fn try_from(v: Vec<(String, u64)>) -> CalyxResult<Self> {
        let mut attrs = LinkedHashMap::with_capacity(v.len());
        for (k, v) in v {
            if attrs.contains_key(&k) {
                return Err(Self::Error::malformed_structure(format!(
                    "Multiple entries for attribute: {k}"
                )));
            }
            attrs.insert(k, v);
        }
        Ok(Attributes { attrs, span: None })
    }
}

impl WithPos for Attributes {
    fn copy_span(&self) -> Option<Span> {
        self.span.clone()
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

    /// Returns true if there are no attributes
    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty()
    }

    /// Remove attribute with the name `key`
    pub fn remove<S>(&mut self, key: S) -> Option<u64>
    where
        S: ToString,
    {
        self.attrs.remove(&key.to_string())
    }

    /// Iterate over all attributes
    pub fn iter(&self) -> impl Iterator<Item = (&String, &u64)> {
        self.attrs.iter()
    }

    /// Set the span information
    pub fn add_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

impl<T: GetAttributes> WithPos for T {
    fn copy_span(&self) -> Option<Span> {
        self.get_attributes().and_then(|attrs| attrs.copy_span())
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
