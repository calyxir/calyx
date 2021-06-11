use crate::values::{OutputValue, TimeLockedValue, Value};
use calyx::ir::{Assignment, Cell, Port, RRC};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
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

pub enum OutputValueRef<'a> {
    ImmediateValue(&'a Value),
    LockedValue(&'a TimeLockedValue),
}

impl<'a> From<&'a Value> for OutputValueRef<'a> {
    fn from(input: &'a Value) -> Self {
        Self::ImmediateValue(input)
    }
}

impl<'a> From<&'a OutputValue> for OutputValueRef<'a> {
    fn from(input: &'a OutputValue) -> Self {
        match input {
            OutputValue::ImmediateValue(val) => Self::ImmediateValue(val),
            OutputValue::LockedValue(val) => Self::LockedValue(val),
        }
    }
}
