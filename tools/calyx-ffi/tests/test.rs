use calyx_ffi::prelude::*;

use calyx_ffi::useless_ffi_backend;

#[calyx_ffi(
    src = "file.futil",
    comp = "main",
  backend = useless_ffi_backend
)]
struct Main;

#[test]
fn test() {
    let mut main = Main::default();
    assert!(main.name() == "main");
    main.reset();
    assert!(main.reset == 0);
    main.tick();
}
