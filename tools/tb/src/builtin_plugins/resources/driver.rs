use calyx_ffi::prelude::*;
use test_crate::calyx_ffi_generated_top::run_tests;

fn main() {
    let mut ffi = CalyxFFI::default();
    unsafe {
        run_tests(&mut ffi);
    }
}
