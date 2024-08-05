use calyx_ffi::declare_calyx_interface;
use calyx_ffi::prelude::*;

use calyx_ffi::cider_ffi_backend;

// not necessary, just to show it off
declare_calyx_interface! {
    In2Out1(lhs, rhs) -> (result)
}

#[calyx_ffi(
    src = "/Users/ethan/Documents/GitHub/calyx/tools/tb/examples/calyx/adder.futil",
    comp = "main",
    backend = cider_ffi_backend,
    derive = [
        In2Out1(lhs, rhs) -> (result)
    ]
)]
struct Adder;

#[cfg(test)]
#[calyx_ffi_tests]
mod tests {
    use super::*;

    fn add<I: In2Out1>(adder: &mut I, lhs: u64, rhs: u64) -> u64 {
        *adder.lhs() = lhs;
        *adder.rhs() = rhs;
        adder.go();
        adder.result()
    }

    #[calyx_ffi_test]
    fn test(adder: &mut Adder) {
        println!("testing adder");
        adder.reset();
        for i in 0..10 {
            for j in 0..10 {
                assert!(add(adder, i, j) == i + j);
            }
        }
    }
}
