use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Alias for a RefCell contained in an Rc reference.
pub type RRC<T> = Rc<RefCell<T>>;

/// A Wrapper for a weak RefCell pointer.
/// Used by parent pointers in the internal representation.
#[derive(Debug)]
pub struct WRC<T> {
    pub(super) internal: Weak<RefCell<T>>,
}

impl<T> WRC<T> {
    /// Convinience method to upgrade and extract the underlying internal weak
    /// pointer.
    pub fn upgrade(&self) -> RRC<T> {
        self.internal
            .upgrade()
            .expect("Weak reference points to nothing")
    }
}

/// From implementation with the same signature as `Rc::downgrade`.
impl<T> From<&RRC<T>> for WRC<T> {
    fn from(internal: &RRC<T>) -> Self {
        Self {
            internal: Rc::downgrade(&internal),
        }
    }
}

/// Clone the Weak reference inside the WRC.
impl<T> Clone for WRC<T> {
    fn clone(&self) -> Self {
        Self {
            internal: Weak::clone(&self.internal),
        }
    }
}
