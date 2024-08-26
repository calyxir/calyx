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

#[calyx_ffi(
    src = "/Users/ethan/Documents/GitHub/calyx/tools/tb/examples/calyx/subber.futil",
    comp = "main",
    backend = cider_ffi_backend,
    derive = [
        In2Out1(lhs, rhs) -> (result)
    ]
)]
struct Subber;

#[cfg(test)]
#[calyx_ffi_tests]
mod tests {
    use super::*;
    use rand::Rng;
    use std::mem;

    // inv: the left argument will always be greater than the right
    fn fuzz_in2out1<I: In2Out1, F: Fn(u64, u64) -> u64>(
        comp: &mut I,
        oracle: &F,
    ) {
        comp.reset();
        let mut rng = rand::thread_rng();
        for (mut x, mut y) in (0..100).map(|_| (rng.gen(), rng.gen())) {
            if y > x {
                mem::swap(&mut x, &mut y);
            }
            *comp.lhs() = x;
            *comp.rhs() = y;
            comp.go();
            assert_eq!(oracle(x, y), comp.result(), "testing f({}, {})", x, y);
        }
    }

    #[calyx_ffi_test]
    fn test_add(adder: &mut Adder) {
        println!("testing adder");
        fuzz_in2out1(adder, &|x, y| x.wrapping_add(y))
    }

    #[calyx_ffi_test]
    fn test_sub(subber: &mut Subber) {
        println!("testing subber");
        fuzz_in2out1(subber, &|x, y| x - y)
    }
}
