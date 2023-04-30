use super::Attribute;
use calyx_utils::{CalyxResult, GPosIdx, Id, WithPos};
use linked_hash_map::LinkedHashMap;
use std::{
    convert::TryFrom,
    ops::{Index, IndexMut},
};

/// Attributes associated with a specific IR structure.
#[derive(Debug, Clone)]
pub struct Attributes {
    /// Mapping from the name of the attribute to its value.
    pub(super) attrs: LinkedHashMap<Id, u64>,
    /// Source location information for the item
    span: GPosIdx,
}

impl IntoIterator for Attributes {
    type Item = (Id, u64);
    type IntoIter = linked_hash_map::IntoIter<Id, u64>;

    fn into_iter(self) -> Self::IntoIter {
        self.attrs.into_iter()
    }
}
impl<'a> IntoIterator for &'a Attributes {
    type Item = (&'a Id, &'a u64);
    type IntoIter = linked_hash_map::Iter<'a, Id, u64>;

    fn into_iter(self) -> Self::IntoIter {
        self.attrs.iter()
    }
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            // Does not allocate any space.
            attrs: LinkedHashMap::with_capacity(0),
            span: GPosIdx::UNKNOWN,
        }
    }
}

impl TryFrom<Vec<(Id, u64)>> for Attributes {
    type Error = calyx_utils::Error;

    fn try_from(v: Vec<(Id, u64)>) -> CalyxResult<Self> {
        let mut attrs = LinkedHashMap::with_capacity(v.len());
        for (k, v) in v {
            if attrs.contains_key(&k) {
                return Err(Self::Error::malformed_structure(format!(
                    "Multiple entries for attribute: {k}"
                )));
            }
            attrs.insert(k, v);
        }
        Ok(Attributes {
            attrs,
            span: GPosIdx::UNKNOWN,
        })
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
        self.attrs.insert(key.into(), val);
    }

    /// Get the value associated with an attribute key
    pub fn get(&self, key: Attribute) -> Option<&u64> {
        self.attrs.get(&key.into())
    }

    /// Check if an attribute key has been set
    pub fn has(&self, key: Attribute) -> bool {
        self.attrs.contains_key(&key.into())
    }

    /// Returns true if there are no attributes
    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty()
    }

    /// Remove attribute with the name `key`
    pub fn remove<S>(&mut self, key: S) -> Option<u64>
    where
        S: Into<Id>,
    {
        self.attrs.remove(&key.into())
    }

    /// Set the span information
    pub fn add_span(mut self, span: GPosIdx) -> Self {
        self.span = span;
        self
    }
}

impl Index<Attribute> for Attributes {
    type Output = u64;

    fn index(&self, key: Attribute) -> &u64 {
        self.get(key).unwrap_or_else(|| {
            panic!("No key `{}` in attribute map", key.to_string())
        })
    }
}

impl IndexMut<Attribute> for Attributes {
    fn index_mut(&mut self, index: Attribute) -> &mut Self::Output {
        let key = index.into();
        self.attrs.insert(key, 0);
        self.attrs.get_mut(&key).unwrap()
    }
}
