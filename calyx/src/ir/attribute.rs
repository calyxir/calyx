use linked_hash_map::LinkedHashMap;
use serde::Serialize;
use serde_with::{serde_as, SerializeAs};
use std::{
    convert::TryFrom,
    ops::{Index, IndexMut},
};

use crate::{
    errors::CalyxResult,
    utils::{GPosIdx, WithPos},
};

use super::Id;

struct SerLinkedHashMapIdu64;

impl SerializeAs<LinkedHashMap<Id, u64>> for SerLinkedHashMapIdu64 {
    fn serialize_as<S>(
        value: &LinkedHashMap<Id, u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_map(value.iter())
    }
}

/// Attributes associated with a specific IR structure.
#[serde_as]
#[derive(Debug, Clone, Serialize)]
pub struct Attributes {
    /// Mapping from the name of the attribute to its value.
    #[serde_as(as = "SerLinkedHashMapIdu64")]
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
    type Error = crate::errors::Error;

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
    #[inline]
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
    pub fn insert<S>(&mut self, key: S, val: u64)
    where
        S: Into<Id>,
    {
        self.attrs.insert(key.into(), val);
    }

    /// Get the value associated with an attribute key
    pub fn get<S>(&self, key: S) -> Option<&u64>
    where
        S: Into<Id>,
    {
        self.attrs.get(&key.into())
    }

    /// Check if an attribute key has been set
    pub fn has<S>(&self, key: S) -> bool
    where
        S: Into<Id>,
    {
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

impl<T: GetAttributes> WithPos for T {
    fn copy_span(&self) -> GPosIdx {
        self.get_attributes().copy_span()
    }
}

impl<S> Index<S> for Attributes
where
    S: Into<Id>,
{
    type Output = u64;

    fn index(&self, key: S) -> &u64 {
        let idx = key.into();
        self.get(idx)
            .unwrap_or_else(|| panic!("No key `{}` in attribute map", idx))
    }
}

impl<S> IndexMut<S> for Attributes
where
    S: Into<Id>,
{
    fn index_mut(&mut self, index: S) -> &mut Self::Output {
        let key = index.into();
        self.attrs.insert(key, 0);
        self.attrs.get_mut(&key).unwrap()
    }
}
