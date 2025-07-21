use super::Attribute;
use crate::{InlineAttributes, attribute::SetAttribute};
use calyx_utils::{CalyxResult, GPosIdx, WithPos};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use smallvec::SmallVec;
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
};

#[derive(Debug, Clone, Default)]
/// Attribute information stored on the Heap
struct HeapAttrInfo {
    attrs: LinkedHashMap<Attribute, u64>,
    set_attrs: HashMap<SetAttribute, VecSet<u32, 4>>,
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

pub enum ParseAttributeWrapper {
    Attribute(Attribute, u64),
    Set(SetAttribute, Vec<u32>),
}

impl From<(Attribute, u64)> for ParseAttributeWrapper {
    fn from(value: (Attribute, u64)) -> Self {
        Self::Attribute(value.0, value.1)
    }
}

impl From<(SetAttribute, Vec<u32>)> for ParseAttributeWrapper {
    fn from(value: (SetAttribute, Vec<u32>)) -> Self {
        Self::Set(value.0, value.1)
    }
}

impl TryFrom<Vec<ParseAttributeWrapper>> for Attributes {
    type Error = calyx_utils::Error;

    fn try_from(v: Vec<ParseAttributeWrapper>) -> CalyxResult<Self> {
        let mut attrs = Attributes::default();

        for item in v {
            match item {
                ParseAttributeWrapper::Attribute(k, v) => {
                    if attrs.has(k) {
                        return Err(Self::Error::malformed_structure(format!(
                            "Multiple entries for attribute: {}",
                            k
                        )));
                    }
                    attrs.insert(k, v);
                }
                ParseAttributeWrapper::Set(set_attr, vec) => {
                    if attrs.hinfo.set_attrs.contains_key(&set_attr) {
                        return Err(Self::Error::malformed_structure(format!(
                            "Multiple entries for attribute: {}",
                            set_attr
                        )));
                    }

                    attrs
                        .hinfo
                        .set_attrs
                        .insert(set_attr, vec.into_iter().collect());
                }
            }
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
    pub fn insert_set<S>(&mut self, key: S, val: u32)
    where
        S: Into<SetAttribute>,
    {
        self.hinfo
            .set_attrs
            .entry(key.into())
            .or_default()
            .insert(val);
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

    pub fn get_set<S>(&self, key: S) -> Option<&VecSet<u32>>
    where
        S: Into<SetAttribute>,
    {
        self.hinfo.set_attrs.get(&key.into())
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
        self.inl.is_empty()
            && self.hinfo.attrs.is_empty()
            && self.hinfo.set_attrs.is_empty()
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

    /// Copies the values of the given set attributes,`keys`, from `other`
    /// into `self`. Note that this does not overwrite set values in `self` that
    /// are already present.
    pub fn copy_from_set<A>(&mut self, other: &Self, keys: Vec<A>)
    where
        A: Into<SetAttribute> + Clone,
    {
        for key in keys {
            if let Some(vals) = other.get_set(key.clone()) {
                self.hinfo
                    .set_attrs
                    .entry(key.clone().into())
                    .or_default()
                    .extend(vals.iter().cloned());
            }
        }
    }

    /// Set the span information
    pub fn add_span(mut self, span: GPosIdx) -> Self {
        self.hinfo.span = span;
        self
    }

    pub fn to_string_with<F, S>(
        &self,
        sep: &'static str,
        fmt: F,
        set_fmt: S,
    ) -> String
    where
        F: Fn(String, u64) -> String,
        S: Fn(String, &[u32]) -> String,
    {
        if self.is_empty() {
            return String::default();
        }

        self.hinfo
            .attrs
            .iter()
            .map(|(k, v)| fmt(k.to_string(), *v))
            .chain(self.inl.iter().map(|k| fmt(k.as_ref().to_string(), 1)))
            .chain(
                self.hinfo
                    .set_attrs
                    .iter()
                    .sorted_by_key(|(k, _)| *k)
                    .filter_map(|(k, v)| {
                        if v.is_empty() {
                            None
                        } else {
                            let formatted =
                                set_fmt(k.to_string(), v.as_slice());
                            if formatted.is_empty() {
                                None
                            } else {
                                Some(formatted)
                            }
                        }
                    }),
            )
            .collect::<Vec<_>>()
            .join(sep)
    }
}

impl PartialEq for Attributes {
    fn eq(&self, other: &Self) -> bool {
        self.inl == other.inl
            && self.hinfo.attrs.len() == other.hinfo.attrs.len()
            && self
                .hinfo
                .attrs
                .iter()
                .all(|(k, v)| other.hinfo.attrs.get(k) == Some(v))
            && self
                .hinfo
                .set_attrs
                .iter()
                .all(|(k, v)| other.hinfo.set_attrs.get(k) == Some(v))
    }
}

impl Eq for Attributes {}

#[cfg(feature = "serialize")]
impl serde::Serialize for HeapAttrInfo {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ser.collect_map(self.to_owned().attrs.iter())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VecSet<D, const ALLOC: usize = 4>
where
    D: Eq + std::hash::Hash + Clone,
{
    inner: SmallVec<[D; ALLOC]>,
}

impl<D, const ALLOC: usize> Extend<D> for VecSet<D, ALLOC>
where
    D: Eq + std::hash::Hash + Clone,
{
    fn extend<T: IntoIterator<Item = D>>(&mut self, iter: T) {
        let mut set: HashSet<_> = self.iter().cloned().collect();
        for i in iter {
            if set.insert(i.clone()) {
                self.inner.push(i);
            }
        }
    }
}

impl<D, const ALLOC: usize> VecSet<D, ALLOC>
where
    D: Eq + std::hash::Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            inner: SmallVec::new(),
        }
    }

    pub fn insert(&mut self, d: D) {
        if !self.inner.contains(&d) {
            self.inner.push(d);
        }
    }

    pub fn contains(&self, d: &D) -> bool {
        self.inner.contains(d)
    }

    pub fn iter(&self) -> impl Iterator<Item = &D> {
        self.inner.iter()
    }

    pub fn as_slice(&self) -> &[D] {
        self.inner.as_slice()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<D, const ALLOC: usize> FromIterator<D> for VecSet<D, ALLOC>
where
    D: Eq + std::hash::Hash + Clone,
{
    fn from_iter<T: IntoIterator<Item = D>>(iter: T) -> Self {
        Self {
            inner: iter.into_iter().unique().collect(),
        }
    }
}
