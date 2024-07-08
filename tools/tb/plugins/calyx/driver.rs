use calyx_ffi::prelude::*;
use test_crate::calyx_ffi_test;

fn main() {
    let mut ffi = CalyxFFI::default();
    unsafe {
        calyx_ffi_test(&mut ffi);
    }
}
