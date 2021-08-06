use crate::values::Value;
use calyx::errors::Error;
use calyx::ir::Binding;
use calyx::ir::{Assignment, Id, Port, RRC};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

/// A wrapper to enable hashing of assignments by their destination port.
pub(super) struct PortAssignment<'a>(*const Port, &'a Assignment);

impl<'a> Hash for PortAssignment<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<'a> PartialEq for PortAssignment<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl<'a> Eq for PortAssignment<'a> {}

impl<'a, 'b> PortAssignment<'a> {
    /// Construct a new PortAssignment.
    pub fn new(p_ref: &'b Port, a_ref: &'a Assignment) -> Self {
        Self(p_ref as *const Port, a_ref)
    }

    /// Get the associated port.
    pub fn get_port(&self) -> *const Port {
        self.0
    }

    /// Get the associated assignment.
    pub fn get_assignment(&self) -> &Assignment {
        &self.1
    }
}

/// Represent the RRC input as a raw pointer.
pub fn get_const_from_rrc<T>(input: &RRC<T>) -> *const T {
    input.as_ptr()
}

/// A map representing all the identifiers and its associated values in a
/// Futil program.
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct MemoryMap(HashMap<Id, Vec<Value>>);

impl MemoryMap {
    pub fn inflate_map(path: &Option<PathBuf>) -> Result<Option<Self>, Error> {
        if let Some(path) = path {
            let v = fs::read(path)?;
            let file_contents = std::str::from_utf8(&v)?;
            let map: MemoryMap = serde_json::from_str(file_contents).unwrap();
            return Ok(Some(map));
        }

        Ok(None)
    }
}

impl Deref for MemoryMap {
    type Target = HashMap<Id, Vec<Value>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MemoryMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Construct memory bindings.
pub fn construct_bindings<'a, I, S: 'a>(iter: I) -> Binding
where
    S: AsRef<str>,
    I: Iterator<Item = &'a (S, u64)>,
{
    let mut vec = Binding::new();
    for (name, val) in iter {
        vec.push((name.as_ref().into(), *val))
    }
    vec
}
