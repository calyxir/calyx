/// Example FFI backend.
#[macro_export]
macro_rules! useless_ffi_backend {
    (@user_data_type) => {
        () // unit type
    };
    (@init $dut:ident, $ctx:expr; $($input:ident),*; $($output:ident),*) => {
        println!("useless_ffi_backend init");
    };
    (@reset $dut:ident; $($input:ident),*; $($output:ident),*) => {
        println!("useless_ffi_backend reset");
        $dut.done = 0;
        $dut.reset = 1;
        for i in 0..5 {
            $dut.tick();
        }
        $dut.reset = 0;
    };
    (@can_tick $dut:ident; $($input:ident),*; $($output:ident),*) => {
        true
    };
    (@tick $dut:ident; $($input:ident),*; $($output:ident),*) => {
        println!("useless_ffi_backend tick");
        if $dut.done == 1 {
            $dut.done = 0;
        }
    };
    (@go $dut:ident; $($input:ident),*; $($output:ident),*) => {
        println!("useless_ffi_backend go");
        $dut.go = 1;
        $dut.go = 0;
        $dut.done = 1;
        $dut.tick();
    };
}
