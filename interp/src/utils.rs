use crate::values::Value;
use calyx::errors::Error;
use calyx::ir::{self, Assignment, Binding, Id, Port, RRC};
use serde::Deserialize;
use std::cell::Ref;
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
    pub fn new(a_ref: &'a Assignment) -> Self {
        Self(a_ref.dst.as_raw(), a_ref)
    }

    /// Get the associated port.
    pub fn get_port(&self) -> *const Port {
        self.0
    }

    /// Get the associated assignment.
    pub fn get_assignment(&self) -> &Assignment {
        self.1
    }
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

pub trait AsRaw<Target> {
    fn as_raw(&self) -> *const Target;
}

impl<T> AsRaw<T> for &T {
    fn as_raw(&self) -> *const T {
        *self as *const T
    }
}

impl<T> AsRaw<T> for *const T {
    fn as_raw(&self) -> *const T {
        *self
    }
}

impl<'a, T> AsRaw<T> for &Ref<'a, T> {
    fn as_raw(&self) -> *const T {
        self as &T as *const T
    }
}
impl<'a, T> AsRaw<T> for Ref<'a, T> {
    fn as_raw(&self) -> *const T {
        self as &T as *const T
    }
}

impl<T> AsRaw<T> for *mut T {
    fn as_raw(&self) -> *const T {
        *self as *const T
    }
}

impl<T> AsRaw<T> for RRC<T> {
    fn as_raw(&self) -> *const T {
        self.as_ptr()
    }
}

impl<T> AsRaw<T> for &RRC<T> {
    fn as_raw(&self) -> *const T {
        self.as_ptr()
    }
}

pub fn assignment_to_string(assignment: &ir::Assignment) -> String {
    let mut str = vec![];
    ir::IRPrinter::write_assignment(assignment, 0, &mut str)
        .expect("Write Failed");
    String::from_utf8(str).expect("Found invalid UTF-8")
}
