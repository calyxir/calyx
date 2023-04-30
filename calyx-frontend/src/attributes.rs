use crate::InlineAttributes;

use super::Attribute;
use calyx_utils::{CalyxResult, GPosIdx, WithPos};
use linked_hash_map::LinkedHashMap;
use std::convert::TryFrom;

/// Attributes associated with a specific IR structure.
#[derive(Default, Debug, Clone)]
pub struct Attributes {
    /// Mapping from the name of the attribute to its value.
    pub(super) attrs: Box<LinkedHashMap<Attribute, u64>>,
    /// Inlined attributes
    inl: InlineAttributes,
    /// Source location information for the item
    span: GPosIdx,
}

impl IntoIterator for Attributes {
    type Item = (Attribute, u64);
    type IntoIter = linked_hash_map::IntoIter<Attribute, u64>;

    fn into_iter(self) -> Self::IntoIter {
        self.attrs.into_iter()
    }
}
impl<'a> IntoIterator for &'a Attributes {
    type Item = (&'a Attribute, &'a u64);
    type IntoIter = linked_hash_map::Iter<'a, Attribute, u64>;

    fn into_iter(self) -> Self::IntoIter {
        self.attrs.iter()
    }
}

impl TryFrom<Vec<(Attribute, u64)>> for Attributes {
    type Error = calyx_utils::Error;

    fn try_from(v: Vec<(Attribute, u64)>) -> CalyxResult<Self> {
        let mut attrs = Attributes::default();
        for (k, v) in v {
            if attrs.has(k) {
                return Err(Self::Error::malformed_structure(format!(
                    "Multiple entries for attribute: {}",
                    k.to_string()
                )));
            }
            attrs.insert(k, v);
        }
        Ok(attrs)
    }
}

impl WithPos for Attributes {
    fn copy_span(&self) -> GPosIdx {
        self.span
    }
}

/// Structs that can return an [`Attributes`] instance.
pub trait GetAttributes {
    /// Returns an [`Attributes`] instance
    fn get_attributes(&self) -> &Attributes;

    /// Returns a mutable [`Attributes`] instance
    fn get_mut_attributes(&mut self) -> &mut Attributes;
}

impl Attributes {
    /// Add a new attribute
    pub fn insert(&mut self, key: Attribute, val: u64) {
        if key.is_inline() {
            assert!(
                val == 1,
                "{} is a unit attribute and cannot have a value",
                key.to_string(),
            );
            return self.inl.insert(key);
        }
        self.attrs.insert(key, val);
    }

    /// Get the value associated with an attribute key
    pub fn get(&self, key: Attribute) -> Option<u64> {
        if key.is_inline() && self.inl.has(key) {
            return Some(1);
        }
        self.attrs.get(&key).cloned()
    }

    /// Check if an attribute key has been set
    pub fn has(&self, key: Attribute) -> bool {
        if key.is_inline() {
            return self.inl.has(key);
        }
        self.attrs.contains_key(&key)
    }

    /// Returns true if there are no attributes
    pub fn is_empty(&self) -> bool {
        self.inl.is_empty() && self.attrs.is_empty()
    }

    /// Remove attribute with the name `key`
    pub fn remove(&mut self, key: Attribute) {
        if key.is_inline() {
            return self.inl.remove(key);
        }
        self.attrs.remove(&key);
    }

    /// Set the span information
    pub fn add_span(mut self, span: GPosIdx) -> Self {
        self.span = span;
        self
    }
}
