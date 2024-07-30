use calyx_ir::Context;
use interp::flatten::structures::{
    context::Context as CiderContext,
    environment::{Environment, Simulator},
};
use std::{mem::MaybeUninit, rc::Rc};

pub struct CiderFFIBackend {
    simulator: Simulator<Rc<CiderContext>>,
}

impl CiderFFIBackend {
    pub fn from(context: &Context, name: &'static str) -> Self {
        let cider_context = CiderContext::new();
        let environment = Environment::new(Rc::new(cider_context), None);
        let simulator = Simulator::new(environment);
        Self { simulator }
    }

    pub fn write_port(&mut self, name: &'static str, value: u64) {
        todo!("no way to set port on a component yet I think")
        // self.simulator.
    }

    pub fn read_port(&self, name: &'static str) -> u64 {
        todo!("no way to get port on a component yet I think")
    }

    pub fn step(&mut self) {
        self.simulator.step().expect(
            "this function isn't documented so don't know what went wrong",
        );
    }
}

/// Runs the component using cider2.
#[macro_export]
macro_rules! cider_ffi_backend {
    (@user_data_type) => {
        $crate::backend::cider::CiderFFIBackend
    };
    (@init $dut:ident, $ctx:expr; $($port:ident),*) => {
        $dut.user_data
            .write($crate::backend::cider::CiderFFIBackend::from(
                $ctx,
                $dut.name(),
            ));
    };
    (@reset $dut:ident; $($port:ident),*) => {
        println!("cider_ffi_backend reset");
        $dut.done = 0;
        $dut.reset = 1;
        for i in 0..5 {
            $dut.tick();
        }
        $dut.reset = 0;
    };
    (@can_tick $dut:ident; $($port:ident),*) => {
        true
    };
    (@tick $dut:ident; $($port:ident),*) => {
        println!("cider_ffi_backend tick");
        let cider = unsafe { $dut.user_data.assume_init_mut() };
        $(
            cider.write_port(stringify!($port), $dut.$port);
        )*
        cider.step();
        $(
            $dut.$port = cider.read_port(stringify!($port));
        )*
    };
    (@go $dut:ident; $($port:ident),*) => {
        println!("cider_ffi_backend go");
        $dut.go = 1;
        while ($dut.done != 1) {
            $dut.tick();
        }
        $dut.go = 0;
    };
}
