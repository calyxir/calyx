use super::Attribute;
use crate::InlineAttributes;
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
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
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
                    k
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
    pub fn insert<A>(&mut self, key: A, val: u64)
    where
        A: Into<Attribute>,
    {
        match key.into() {
            Attribute::Bool(b) => {
                assert!(
                    val == 1,
                    "{} is a boolean attribute and can only have a value of 1",
                    b.as_ref(),
                );
                self.inl.insert(b);
            }
            attr => {
                self.hinfo.attrs.insert(attr, val);
            }
        }
    }

    /// Get the value associated with an attribute key
    pub fn get<A>(&self, key: A) -> Option<u64>
    where
        A: Into<Attribute>,
    {
        match key.into() {
            Attribute::Bool(b) => {
                if self.inl.has(b) {
                    Some(1)
                } else {
                    None
                }
            }
            attr => self.hinfo.attrs.get(&attr).cloned(),
        }
    }

    /// Check if an attribute key has been set
    pub fn has<A>(&self, key: A) -> bool
    where
        A: Into<Attribute>,
    {
        match key.into() {
            Attribute::Bool(b) => self.inl.has(b),
            attr => self.hinfo.attrs.contains_key(&attr),
        }
    }

    /// Returns true if there are no attributes
    pub fn is_empty(&self) -> bool {
        self.inl.is_empty() && self.hinfo.attrs.is_empty()
    }

    /// Remove attribute with the name `key`
    pub fn remove<A>(&mut self, key: A)
    where
        A: Into<Attribute>,
    {
        match key.into() {
            Attribute::Bool(b) => {
                self.inl.remove(b);
            }
            attr => {
                self.hinfo.attrs.remove(&attr);
            }
        }
    }

    /// `self` copys (i.e., assigns the same values) the attributes in `other`.
    /// However, we only copy attributes in `keys` (i.e.. we don't copy
    /// all attributes in `other`, only the ones that we specify).
    /// If a `key` is not present in `other`, then we ignore that `key`.
    /// Example: suppose
    /// self: A->10, B->5
    /// other: A->15, C->5
    /// keys: A, D
    /// Then self gets: A->15 B->5. (D is ignored since it's not present in other
    /// and C is ignored since it's not keys.)
    pub fn copy_from<A>(&mut self, other: Self, keys: Vec<A>)
    where
        A: Into<Attribute> + Clone,
    {
        for key in keys {
            match other.get(key.clone()) {
                None => (),
                Some(val) => self.insert(key, val),
            }
        }
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
            .chain(self.inl.iter().map(|k| fmt(k.as_ref().to_string(), 1)))
            .collect::<Vec<_>>()
            .join(sep)
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for HeapAttrInfo {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ser.collect_map(self.to_owned().attrs.iter())
    }
}
