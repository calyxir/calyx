/// Example FFI backend.
#[macro_export]
macro_rules! useless_ffi_backend {
    (init $dut:ident; $($port:ident),*) => {
        println!("useless_ffi_backend init");
    };
    (deinit $dut:ident; $($port:ident),*) => {
        println!("useless_ffi_backend deinit");
    };
    (reset $dut:ident; $($port:ident),*) => {
        println!("useless_ffi_backend reset");
        $dut.done = 0;
        $dut.reset = 1;
        for i in 0..5 {
            $dut.tick();
        }
        $dut.reset = 0;
    };
    (tick $dut:ident; $($port:ident),*) => {
        println!("useless_ffi_backend tick");
        if $dut.done == 1 {
            $dut.done = 0;
        }
    };
    (go $dut:ident; $($port:ident),*) => {
        println!("useless_ffi_backend go");
        $dut.go = 1;
        $dut.go = 0;
        $dut.done = 1;
        $dut.tick();
    };
}
