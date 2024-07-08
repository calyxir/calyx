use calyx_ffi::prelude::*;

#[calyx_ffi(
    src = "/Users/ethan/Documents/GitHub/calyx/tools/calyx-ffi/tests/file.futil",
    comp = "main",
    backend = useless_ffi_backend
)]
struct Main;

#[test]
fn test() {
    let mut main = Main::default();
    main.reset();
    assert!(main.reset == 0);
    main.tick();
}
