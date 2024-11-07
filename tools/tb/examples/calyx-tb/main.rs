use calyx_ffi::prelude::*;

use calyx_ffi::cider_ffi_backend;
#[calyx_ffi(
    src = "adder.futil",
    comp = "main",
    backend = cider_ffi_backend
)]
struct Adder;

#[cfg(test)]
#[calyx_ffi_tests]
mod tests {
    #[calyx_ffi_test]
    fn test_add(adder: &mut Adder) {
        adder.reset();
        adder.lhs = 4;
        adder.rhs = 5;
        println!("foo");
        adder.go();
        assert_eq!(9, adder.result());
    }
}
