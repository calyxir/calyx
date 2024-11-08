use calyx_ir::Context;
use interp::{
    configuration::RuntimeConfig,
    flatten::{
        flat_ir,
        structures::{
            context::Context as CiderContext, environment::Simulator,
        },
    },
    BitVecValue,
};
use std::rc::Rc;

pub struct CiderFFIBackend {
    simulator: Simulator<Rc<CiderContext>>,
}

impl CiderFFIBackend {
    pub fn from(ctx: &Context, _name: &'static str) -> Self {
        // TODO(ethan, maybe griffin): use _name to select the component somehow
        let ctx = flat_ir::translate(ctx);
        let simulator = Simulator::build_simulator(
            Rc::new(ctx),
            &None,
            &None,
            RuntimeConfig::default(),
        )
        .expect("we live on the edge");
        Self { simulator }
    }

    pub fn write_port(&mut self, name: &'static str, value: &BitVecValue) {
        if name == "go" || name == "reset" {
            return;
        }
        self.simulator.pin_value(name, value.clone());
    }

    pub fn read_port(&self, name: &'static str) -> BitVecValue {
        self.simulator
            .lookup_port_from_string(&String::from(name))
            .expect("wrong port name")
    }

    pub fn step(&mut self) {
        self.simulator.step().expect(
            "this function isn't documented so don't know what went wrong",
        );
    }

    pub fn go(&mut self) {
        self.simulator.run_program().expect("failed to run program");
        self.step(); // since griffin said so
    }
}

/// Runs the component using cider2.
#[macro_export]
macro_rules! cider_ffi_backend {
    (@user_data_type) => {
        $crate::backend::cider::CiderFFIBackend
    };
    (@init $dut:ident, $ctx:expr; $($input:ident),*; $($output:ident),*) => {
        $dut.user_data
            .write($crate::backend::cider::CiderFFIBackend::from(
                $ctx,
                $dut.name(),
            ));
    };
    (@reset $dut:ident; $($input:ident),*; $($output:ident),*) => {
        println!("cider_ffi_backend reset. doesn't work LOL");
        // $dut.done = 0;
        // $dut.reset = 1;
        // for i in 0..5 {
        //     $dut.tick();
        // }
        // $dut.reset = 0;
    };
    (@can_tick $dut:ident; $($input:ident),*; $($output:ident),*) => {
        true
    };
    (@tick $dut:ident; $($input:ident),*; $($output:ident),*) => {
        // println!("cider_ffi_backend tick");
        let cider = unsafe { $dut.user_data.assume_init_mut() };
        $(
            cider.write_port(stringify!($input), &$dut.$input);
        )*
        cider.step();
        $(
            $dut.$output = cider.read_port(stringify!($output));
        )*
    };
    (@go $dut:ident; $($input:ident),*; $($output:ident),*) => {
        // println!("cider_ffi_backend go");
        let cider = unsafe { $dut.user_data.assume_init_mut() };
        $(
            cider.write_port(stringify!($input), &$dut.$input);
        )*
        cider.go();
        $(
            $dut.$output = cider.read_port(stringify!($output));
        )*
    };
}
