use calyx_ir::Context;
use interp::{
    configuration::RuntimeConfig,
    flatten::{
        flat_ir,
        structures::{
            context::Context as CiderContext,
            environment::{BaseSimulator, Environment},
        },
    },
    BitVecValue,
};
use std::rc::Rc;

#[derive(Clone)]
pub struct CiderFFIBackend {
    simulator: BaseSimulator<Rc<CiderContext>>,
}

impl CiderFFIBackend {
    pub fn from(ctx: &Context, _name: &'static str) -> Self {
        // TODO(ethan, maybe griffin): use _name to select the component somehow
        let ctx = flat_ir::translate(ctx);
        let config = RuntimeConfig::default();
        let enviroment = Environment::new(
            Rc::new(ctx),
            None,
            false,
            config.get_logging_config(),
        );
        let simulator = BaseSimulator::new(enviroment, config);
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
        self.simulator
            .run_program_inner(None)
            .expect("failed to run program");
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
            cider.write_port(stringify!($input), &$dut.$input.inner);
        )*
        cider.step();
        $(
            $dut.$output.inner = cider.read_port(stringify!($output));
        )*
    };
    (@go $dut:ident; $($input:ident),*; $($output:ident),*) => {
        // println!("cider_ffi_backend go");
        let cider = unsafe { $dut.user_data.assume_init_mut() };
        $(
            cider.write_port(stringify!($input), &$dut.$input.inner);
        )*
        cider.go();
        $(
            $dut.$output.inner = cider.read_port(stringify!($output));
        )*
    };
}
