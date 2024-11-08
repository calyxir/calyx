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
        $dut.done = interp::BitVecValue::from_u64(0, $dut.done_width() as u32);
        $dut.set_reset(1);
        for i in 0..5 {
            $dut.tick();
        }
        $dut.set_reset(0);
    };
    (@can_tick $dut:ident; $($input:ident),*; $($output:ident),*) => {
        true
    };
    (@tick $dut:ident; $($input:ident),*; $($output:ident),*) => {
        println!("useless_ffi_backend tick");
        if $dut.done() == 1 {
            $dut.set_reset(0);
        }
    };
    (@go $dut:ident; $($input:ident),*; $($output:ident),*) => {
        println!("useless_ffi_backend go");
        $dut.set_go(1);
        $dut.set_go(0);
        $dut.done = interp::BitVecValue::from_u64(1, $dut.done_width() as u32);
        $dut.tick();
    };
}
