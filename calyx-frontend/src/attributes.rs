use crate::InlineAttributes;

use super::Attribute;
use calyx_utils::{CalyxResult, GPosIdx, WithPos};
use linked_hash_map::LinkedHashMap;
use std::convert::TryFrom;

#[derive(Debug, Clone, Default)]
/// Attribute information stored on the Heap
struct HeapAttrInfo {
    attrs: LinkedHashMap<Attribute, u64>,
    span: GPosIdx,
}

/// Attributes associated with a specific IR structure.
#[derive(Default, Debug, Clone)]
pub struct Attributes {
    /// Inlined attributes
    inl: InlineAttributes,
    /// Attributes stored on the heap
    hinfo: Box<HeapAttrInfo>,
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
        self.hinfo.span
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
        self.hinfo.attrs.insert(key, val);
    }

    /// Get the value associated with an attribute key
    pub fn get(&self, key: Attribute) -> Option<u64> {
        if key.is_inline() && self.inl.has(key) {
            return Some(1);
        }
        self.hinfo.attrs.get(&key).cloned()
    }

    /// Check if an attribute key has been set
    pub fn has(&self, key: Attribute) -> bool {
        if key.is_inline() {
            return self.inl.has(key);
        }
        self.hinfo.attrs.contains_key(&key)
    }

    /// Returns true if there are no attributes
    pub fn is_empty(&self) -> bool {
        self.inl.is_empty() && self.hinfo.attrs.is_empty()
    }

    /// Remove attribute with the name `key`
    pub fn remove(&mut self, key: Attribute) {
        if key.is_inline() {
            return self.inl.remove(key);
        }
        self.hinfo.attrs.remove(&key);
    }

    /// Set the span information
    pub fn add_span(mut self, span: GPosIdx) -> Self {
        self.hinfo.span = span;
        self
    }

    pub fn to_string_with<F>(&self, sep: &'static str, fmt: F) -> String
    where
        F: Fn(String, u64) -> String,
    {
        if self.is_empty() {
            return String::default();
        }

        self.hinfo
            .attrs
            .iter()
            .map(|(k, v)| fmt(k.to_string(), *v))
            .chain(self.inl.iter().map(|k| fmt(k.to_string(), 1)))
            .collect::<Vec<_>>()
            .join(sep)
    }
}
