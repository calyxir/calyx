use calyx_ffi::prelude::*;

use calyx_ffi::cider_ffi_backend;

// not necessary, just to show it off
calyx_ffi::declare_interface! {
    In2Out1(lhs: 64, rhs: 64) -> (result: 64)
}

#[calyx_ffi(
    src = "tests/adder.futil",
    comp = "main",
    backend = cider_ffi_backend,
    derive = [
        In2Out1(lhs: 64, rhs: 64) -> (result: 64)
    ]
)]
struct Adder;

#[calyx_ffi(
    src = "tests/subber.futil",
    comp = "main",
    backend = cider_ffi_backend,
    derive = [
        In2Out1(lhs: 64, rhs: 64) -> (result: 64)
    ]
)]
struct Subber;

#[cfg(test)]
#[calyx_ffi_tests]
mod tests {
    use std::mem;

    use super::*;
    use rand::Rng;

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
            comp.set_lhs(x);
            comp.set_rhs(y);
            comp.go();
            assert_eq!(
                oracle(x, y),
                comp.result(),
                "component did not evaluate f({}, {}) = {} correctly",
                x,
                y,
                oracle(x, y)
            );
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
