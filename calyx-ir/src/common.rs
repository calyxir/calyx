use calyx_utils::{GetName, Id};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Alias for a RefCell contained in an Rc reference.
#[allow(clippy::upper_case_acronyms)]
pub type RRC<T> = Rc<RefCell<T>>;

/// A Wrapper for a weak RefCell pointer.
/// Used by parent pointers in the internal representation.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub struct WRC<T>
where
    T: GetName,
{
    pub(super) internal: Weak<RefCell<T>>,
    debug_name: Id,
}

impl<T: GetName> WRC<T> {
    /// Convinience method to upgrade and extract the underlying internal weak
    /// pointer.
    pub fn upgrade(&self) -> RRC<T> {
        let Some(r) = self.internal.upgrade() else {
             unreachable!("weak reference points to a dropped. Original object's name: `{}'", self.debug_name)
        };
        r
    }
}

/// From implementation with the same signature as `Rc::downgrade`.
impl<T: GetName> From<&RRC<T>> for WRC<T> {
    fn from(internal: &RRC<T>) -> Self {
        let debug_name = internal.borrow().name();
        Self {
            internal: Rc::downgrade(internal),
            debug_name,
        }
    }
}

/// Clone the Weak reference inside the WRC.
impl<T: GetName> Clone for WRC<T> {
    fn clone(&self) -> Self {
        Self {
            internal: Weak::clone(&self.internal),
            debug_name: self.debug_name,
        }
    }
}
