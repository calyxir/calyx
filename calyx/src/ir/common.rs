use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Alias for a RefCell contained in a Weak reference.
pub type WRC<T> = Weak<RefCell<T>>;
/// Alias for a RefCell contained in an Rc reference.
pub type RRC<T> = Rc<RefCell<T>>;
