use crate::values::Value;
use calyx::errors::Error;
use calyx::ir::Binding;
use calyx::ir::{Assignment, Cell, Id, Port, RRC};
use serde::Deserialize;
use std::cell::Ref;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::path::PathBuf;
pub(super) struct PortRef(RRC<Port>);

impl Deref for PortRef {
    type Target = RRC<Port>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for PortRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.0.borrow() as &Port as *const Port).hash(state);
    }
}

impl PartialEq for PortRef {
    fn eq(&self, other: &Self) -> bool {
        let self_const: *const Port = &*self.0.borrow();
        let other_const: *const Port = &*other.0.borrow();

        std::ptr::eq(self_const, other_const)
    }
}

impl Eq for PortRef {}

impl From<RRC<Port>> for PortRef {
    fn from(input: RRC<Port>) -> Self {
        Self(input)
    }
}

impl From<&RRC<Port>> for PortRef {
    fn from(input: &RRC<Port>) -> Self {
        Self(input.clone())
    }
}

#[derive(Debug, Clone)]
pub(super) struct AssignmentRef<'a>(&'a Assignment);

impl<'a> Hash for AssignmentRef<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0 as *const Assignment).hash(state);
    }
}

impl<'a> PartialEq for AssignmentRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0 as *const Assignment, other.0 as *const Assignment)
    }
}

impl<'a> Eq for AssignmentRef<'a> {}

impl<'a> From<&'a Assignment> for AssignmentRef<'a> {
    fn from(input: &'a Assignment) -> Self {
        Self(input)
    }
}

impl<'a> Deref for AssignmentRef<'a> {
    type Target = Assignment;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

pub(super) struct CellRef(RRC<Cell>);

impl<'a> From<&'a CellRef> for &'a RRC<Cell> {
    fn from(val: &'a CellRef) -> Self {
        &val.0
    }
}

impl Deref for CellRef {
    type Target = RRC<Cell>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for CellRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.0.borrow() as &Cell as *const Cell).hash(state);
    }
}

impl PartialEq for CellRef {
    fn eq(&self, other: &Self) -> bool {
        let self_const: *const Cell = &*self.0.borrow();
        let other_const: *const Cell = &*other.0.borrow();

        std::ptr::eq(self_const, other_const)
    }
}

impl Eq for CellRef {}

impl From<RRC<Cell>> for CellRef {
    fn from(input: RRC<Cell>) -> Self {
        Self(input)
    }
}

impl From<&RRC<Cell>> for CellRef {
    fn from(input: &RRC<Cell>) -> Self {
        Self(input.clone())
    }
}

//new utility:
pub fn get_const_from_rrc<T>(input: &RRC<T>) -> *const T {
    input.as_ptr()
}

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

use std::ops::DerefMut;

impl DerefMut for MemoryMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

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
