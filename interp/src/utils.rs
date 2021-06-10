use calyx::ir::{Port, RRC};
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
