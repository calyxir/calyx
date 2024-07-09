use calyx_ffi::prelude::*;

#[calyx_ffi(
    src = "/Users/ethan/Documents/GitHub/calyx/tools/tb/examples/calyx/adder.futil",
    comp = "main",
    backend = useless_ffi_backend
)]
struct Adder;

#[cfg(test)]
#[calyx_ffi_tests]
mod tests {
    use super::*;

    fn add(adder: &mut Adder, lhs: u64, rhs: u64) -> u64 {
        adder.lhs = lhs;
        adder.rhs = rhs;
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
