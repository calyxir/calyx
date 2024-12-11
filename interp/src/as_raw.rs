use calyx_ir::RRC;
use std::cell::Ref;

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

impl<T> AsRaw<T> for &Ref<'_, T> {
    fn as_raw(&self) -> *const T {
        self as &T as *const T
    }
}
impl<T> AsRaw<T> for Ref<'_, T> {
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
