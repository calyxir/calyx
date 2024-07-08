use calyx_ffi::prelude::*;

#[calyx_ffi(
    src = "/Users/ethan/Documents/GitHub/calyx/tools/tb/examples/calyx/adder.futil",
    comp = "main"
)]
struct Adder;

#[calyx_ffi_tests]
mod tests {
    use super::*;

    #[calyx_ffi_test]
    fn test(adder: &mut Adder) {
        println!("testing adder");
    }
}
