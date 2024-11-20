#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use calyx_ffi::prelude::*;
use calyx_ffi::cider_ffi_backend;
pub trait In2Out1: CalyxFFIComponent {
    fn lhs_bits(&mut self) -> &mut calyx_ffi::value::Value<64>;
    fn set_lhs(&mut self, value: u64);
    fn rhs_bits(&mut self) -> &mut calyx_ffi::value::Value<64>;
    fn set_rhs(&mut self, value: u64);
    fn result_bits(&self) -> &calyx_ffi::value::Value<64>;
    fn result(&self) -> u64;
}
struct Adder {
    pub lhs: calyx_ffi::value::Value<64u64>,
    pub rhs: calyx_ffi::value::Value<64u64>,
    result: calyx_ffi::value::Value<64u64>,
    pub go: calyx_ffi::value::Value<1u64>,
    pub clk: calyx_ffi::value::Value<1u64>,
    pub reset: calyx_ffi::value::Value<1u64>,
    done: calyx_ffi::value::Value<1u64>,
    user_data: std::mem::MaybeUninit<::calyx_ffi::backend::cider::CiderFFIBackend>,
}
impl Adder {
    pub const fn lhs_width() -> calyx_ffi::value::WidthInt {
        64u64 as calyx_ffi::value::WidthInt
    }
    pub const fn rhs_width() -> calyx_ffi::value::WidthInt {
        64u64 as calyx_ffi::value::WidthInt
    }
    pub const fn result_width() -> calyx_ffi::value::WidthInt {
        64u64 as calyx_ffi::value::WidthInt
    }
    pub const fn go_width() -> calyx_ffi::value::WidthInt {
        1u64 as calyx_ffi::value::WidthInt
    }
    pub const fn clk_width() -> calyx_ffi::value::WidthInt {
        1u64 as calyx_ffi::value::WidthInt
    }
    pub const fn reset_width() -> calyx_ffi::value::WidthInt {
        1u64 as calyx_ffi::value::WidthInt
    }
    pub const fn done_width() -> calyx_ffi::value::WidthInt {
        1u64 as calyx_ffi::value::WidthInt
    }
    pub fn result(&self) -> u64 {
        (&self.result).try_into().expect("port value wider than 64 bits")
    }
    pub const fn result_bits(&self) -> &calyx_ffi::value::Value<64u64> {
        &self.result
    }
    pub fn done(&self) -> u64 {
        (&self.done).try_into().expect("port value wider than 64 bits")
    }
    pub const fn done_bits(&self) -> &calyx_ffi::value::Value<1u64> {
        &self.done
    }
    pub fn set_lhs(&mut self, value: u64) {
        self.lhs = calyx_ffi::value::Value::from(value);
    }
    pub fn set_rhs(&mut self, value: u64) {
        self.rhs = calyx_ffi::value::Value::from(value);
    }
    pub fn set_go(&mut self, value: u64) {
        self.go = calyx_ffi::value::Value::from(value);
    }
    pub fn set_clk(&mut self, value: u64) {
        self.clk = calyx_ffi::value::Value::from(value);
    }
    pub fn set_reset(&mut self, value: u64) {
        self.reset = calyx_ffi::value::Value::from(value);
    }
}
impl std::default::Default for Adder {
    fn default() -> Self {
        Self {
            lhs: calyx_ffi::value::Value::from(0),
            rhs: calyx_ffi::value::Value::from(0),
            result: calyx_ffi::value::Value::from(0),
            go: calyx_ffi::value::Value::from(0),
            clk: calyx_ffi::value::Value::from(0),
            reset: calyx_ffi::value::Value::from(0),
            done: calyx_ffi::value::Value::from(0),
            user_data: unsafe { std::mem::MaybeUninit::zeroed() },
        }
    }
}
impl std::clone::Clone for Adder {
    fn clone(&self) -> Self {
        Self {
            lhs: self.lhs.clone(),
            rhs: self.rhs.clone(),
            result: self.result.clone(),
            go: self.go.clone(),
            clk: self.clk.clone(),
            reset: self.reset.clone(),
            done: self.done.clone(),
            user_data: unsafe {
                std::mem::MaybeUninit::new(self.user_data.assume_init_ref().clone())
            },
        }
    }
}
impl CalyxFFIComponent for Adder {
    fn path(&self) -> &'static str {
        "/Users/ethan/Documents/GitHub/calyx/tools/calyx-ffi/tests/adder.futil"
    }
    fn name(&self) -> &'static str {
        "main"
    }
    fn init(&mut self, context: &calyx_ir::Context) {
        self.user_data
            .write(
                ::calyx_ffi::backend::cider::CiderFFIBackend::from(context, self.name()),
            );
    }
    fn reset(&mut self) {
        {
            ::std::io::_print(
                format_args!("cider_ffi_backend reset. doesn\'t work LOL\n"),
            );
        };
    }
    fn can_tick(&self) -> bool {
        true
    }
    fn tick(&mut self) {
        let cider = unsafe { self.user_data.assume_init_mut() };
        cider.write_port("lhs", &self.lhs.inner);
        cider.write_port("rhs", &self.rhs.inner);
        cider.write_port("go", &self.go.inner);
        cider.write_port("clk", &self.clk.inner);
        cider.write_port("reset", &self.reset.inner);
        cider.step();
        self.result.inner = cider.read_port("result");
        self.done.inner = cider.read_port("done");
    }
    fn go(&mut self) {
        let cider = unsafe { self.user_data.assume_init_mut() };
        cider.write_port("lhs", &self.lhs.inner);
        cider.write_port("rhs", &self.rhs.inner);
        cider.write_port("go", &self.go.inner);
        cider.write_port("clk", &self.clk.inner);
        cider.write_port("reset", &self.reset.inner);
        cider.go();
        self.result.inner = cider.read_port("result");
        self.done.inner = cider.read_port("done");
    }
}
impl In2Out1 for Adder {
    fn result_bits(&self) -> &calyx_ffi::value::Value<64> {
        &self.result
    }
    fn result(&self) -> u64 {
        Self::result(self)
    }
    fn lhs_bits(&mut self) -> &mut calyx_ffi::value::Value<64> {
        &mut self.lhs
    }
    fn set_lhs(&mut self, value: u64) {
        Self::set_lhs(self, value);
    }
    fn rhs_bits(&mut self) -> &mut calyx_ffi::value::Value<64> {
        &mut self.rhs
    }
    fn set_rhs(&mut self, value: u64) {
        Self::set_rhs(self, value);
    }
}
struct Subber {
    pub lhs: calyx_ffi::value::Value<64u64>,
    pub rhs: calyx_ffi::value::Value<64u64>,
    result: calyx_ffi::value::Value<64u64>,
    pub go: calyx_ffi::value::Value<1u64>,
    pub clk: calyx_ffi::value::Value<1u64>,
    pub reset: calyx_ffi::value::Value<1u64>,
    done: calyx_ffi::value::Value<1u64>,
    user_data: std::mem::MaybeUninit<::calyx_ffi::backend::cider::CiderFFIBackend>,
}
impl Subber {
    pub const fn lhs_width() -> calyx_ffi::value::WidthInt {
        64u64 as calyx_ffi::value::WidthInt
    }
    pub const fn rhs_width() -> calyx_ffi::value::WidthInt {
        64u64 as calyx_ffi::value::WidthInt
    }
    pub const fn result_width() -> calyx_ffi::value::WidthInt {
        64u64 as calyx_ffi::value::WidthInt
    }
    pub const fn go_width() -> calyx_ffi::value::WidthInt {
        1u64 as calyx_ffi::value::WidthInt
    }
    pub const fn clk_width() -> calyx_ffi::value::WidthInt {
        1u64 as calyx_ffi::value::WidthInt
    }
    pub const fn reset_width() -> calyx_ffi::value::WidthInt {
        1u64 as calyx_ffi::value::WidthInt
    }
    pub const fn done_width() -> calyx_ffi::value::WidthInt {
        1u64 as calyx_ffi::value::WidthInt
    }
    pub fn result(&self) -> u64 {
        (&self.result).try_into().expect("port value wider than 64 bits")
    }
    pub const fn result_bits(&self) -> &calyx_ffi::value::Value<64u64> {
        &self.result
    }
    pub fn done(&self) -> u64 {
        (&self.done).try_into().expect("port value wider than 64 bits")
    }
    pub const fn done_bits(&self) -> &calyx_ffi::value::Value<1u64> {
        &self.done
    }
    pub fn set_lhs(&mut self, value: u64) {
        self.lhs = calyx_ffi::value::Value::from(value);
    }
    pub fn set_rhs(&mut self, value: u64) {
        self.rhs = calyx_ffi::value::Value::from(value);
    }
    pub fn set_go(&mut self, value: u64) {
        self.go = calyx_ffi::value::Value::from(value);
    }
    pub fn set_clk(&mut self, value: u64) {
        self.clk = calyx_ffi::value::Value::from(value);
    }
    pub fn set_reset(&mut self, value: u64) {
        self.reset = calyx_ffi::value::Value::from(value);
    }
}
impl std::default::Default for Subber {
    fn default() -> Self {
        Self {
            lhs: calyx_ffi::value::Value::from(0),
            rhs: calyx_ffi::value::Value::from(0),
            result: calyx_ffi::value::Value::from(0),
            go: calyx_ffi::value::Value::from(0),
            clk: calyx_ffi::value::Value::from(0),
            reset: calyx_ffi::value::Value::from(0),
            done: calyx_ffi::value::Value::from(0),
            user_data: unsafe { std::mem::MaybeUninit::zeroed() },
        }
    }
}
impl std::clone::Clone for Subber {
    fn clone(&self) -> Self {
        Self {
            lhs: self.lhs.clone(),
            rhs: self.rhs.clone(),
            result: self.result.clone(),
            go: self.go.clone(),
            clk: self.clk.clone(),
            reset: self.reset.clone(),
            done: self.done.clone(),
            user_data: unsafe {
                std::mem::MaybeUninit::new(self.user_data.assume_init_ref().clone())
            },
        }
    }
}
impl CalyxFFIComponent for Subber {
    fn path(&self) -> &'static str {
        "/Users/ethan/Documents/GitHub/calyx/tools/calyx-ffi/tests/subber.futil"
    }
    fn name(&self) -> &'static str {
        "main"
    }
    fn init(&mut self, context: &calyx_ir::Context) {
        self.user_data
            .write(
                ::calyx_ffi::backend::cider::CiderFFIBackend::from(context, self.name()),
            );
    }
    fn reset(&mut self) {
        {
            ::std::io::_print(
                format_args!("cider_ffi_backend reset. doesn\'t work LOL\n"),
            );
        };
    }
    fn can_tick(&self) -> bool {
        true
    }
    fn tick(&mut self) {
        let cider = unsafe { self.user_data.assume_init_mut() };
        cider.write_port("lhs", &self.lhs.inner);
        cider.write_port("rhs", &self.rhs.inner);
        cider.write_port("go", &self.go.inner);
        cider.write_port("clk", &self.clk.inner);
        cider.write_port("reset", &self.reset.inner);
        cider.step();
        self.result.inner = cider.read_port("result");
        self.done.inner = cider.read_port("done");
    }
    fn go(&mut self) {
        let cider = unsafe { self.user_data.assume_init_mut() };
        cider.write_port("lhs", &self.lhs.inner);
        cider.write_port("rhs", &self.rhs.inner);
        cider.write_port("go", &self.go.inner);
        cider.write_port("clk", &self.clk.inner);
        cider.write_port("reset", &self.reset.inner);
        cider.go();
        self.result.inner = cider.read_port("result");
        self.done.inner = cider.read_port("done");
    }
}
impl In2Out1 for Subber {
    fn result_bits(&self) -> &calyx_ffi::value::Value<64> {
        &self.result
    }
    fn result(&self) -> u64 {
        Self::result(self)
    }
    fn lhs_bits(&mut self) -> &mut calyx_ffi::value::Value<64> {
        &mut self.lhs
    }
    fn set_lhs(&mut self, value: u64) {
        Self::set_lhs(self, value);
    }
    fn rhs_bits(&mut self) -> &mut calyx_ffi::value::Value<64> {
        &mut self.rhs
    }
    fn set_rhs(&mut self, value: u64) {
        Self::set_rhs(self, value);
    }
}
#[cfg(test)]
mod tests {
    use std::mem;
    use super::*;
    use rand::Rng;
    fn fuzz_in2out1<I: In2Out1, F: Fn(u64, u64) -> u64>(comp: &mut I, oracle: &F) {
        comp.reset();
        let mut rng = rand::thread_rng();
        for (mut x, mut y) in (0..100).map(|_| (rng.gen(), rng.gen())) {
            if y > x {
                mem::swap(&mut x, &mut y);
            }
            comp.set_lhs(x);
            comp.set_rhs(y);
            comp.go();
            match (&oracle(x, y), &comp.result()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!(
                                    "component did not evaluate f({0}, {1}) = {2} correctly",
                                    x,
                                    y,
                                    oracle(x, y),
                                ),
                            ),
                        );
                    }
                }
            };
        }
    }
    fn test_add(adder: &mut Adder) {
        fn assert_is_calyx_ffi_component<T: CalyxFFIComponent>() {}
        assert_is_calyx_ffi_component::<Adder>();
        {
            ::std::io::_print(format_args!("testing adder\n"));
        };
        fuzz_in2out1(adder, &(|x, y| x.wrapping_add(y)))
    }
    fn test_sub(subber: &mut Subber) {
        fn assert_is_calyx_ffi_component<T: CalyxFFIComponent>() {}
        assert_is_calyx_ffi_component::<Subber>();
        {
            ::std::io::_print(format_args!("testing subber\n"));
        };
        fuzz_in2out1(subber, &(|x, y| x - y))
    }
    pub(crate) mod calyx_ffi_generated_wrappers {
        use super::*;
        pub(crate) const CALYX_FFI_TESTS: &'static [unsafe fn(&mut CalyxFFI) -> ()] = &[
            test_add,
            test_sub,
        ];
        pub(crate) unsafe fn test_add(ffi: &mut CalyxFFI) {
            let dut = ffi.new_comp::<Adder>();
            let dut_ref = &mut *dut.borrow_mut();
            let dut_pointer = dut_ref as *mut dyn CalyxFFIComponent as *mut _
                as *mut Adder;
            let dut_concrete: &mut Adder = &mut *dut_pointer;
            super::test_add(dut_concrete);
        }
        pub(crate) unsafe fn test_sub(ffi: &mut CalyxFFI) {
            let dut = ffi.new_comp::<Subber>();
            let dut_ref = &mut *dut.borrow_mut();
            let dut_pointer = dut_ref as *mut dyn CalyxFFIComponent as *mut _
                as *mut Subber;
            let dut_concrete: &mut Subber = &mut *dut_pointer;
            super::test_sub(dut_concrete);
        }
    }
    pub(crate) mod calyx_ffi_generated_tests {
        use super::*;
        extern crate test;
        #[cfg(test)]
        #[rustc_test_marker = "tests::calyx_ffi_generated_tests::test_add"]
        pub const test_add: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::calyx_ffi_generated_tests::test_add"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "tools/calyx-ffi/tests/arith_fuzz.rs",
                start_line: 64usize,
                start_col: 8usize,
                end_line: 64usize,
                end_col: 16usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::IntegrationTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(test_add()),
            ),
        };
        pub(crate) fn test_add() {
            let mut ffi = CalyxFFI::new();
            unsafe {
                super::calyx_ffi_generated_wrappers::test_add(&mut ffi);
            }
        }
        extern crate test;
        #[cfg(test)]
        #[rustc_test_marker = "tests::calyx_ffi_generated_tests::test_sub"]
        pub const test_sub: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::calyx_ffi_generated_tests::test_sub"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "tools/calyx-ffi/tests/arith_fuzz.rs",
                start_line: 70usize,
                start_col: 8usize,
                end_line: 70usize,
                end_col: 16usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::IntegrationTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(test_sub()),
            ),
        };
        pub(crate) fn test_sub() {
            let mut ffi = CalyxFFI::new();
            unsafe {
                super::calyx_ffi_generated_wrappers::test_sub(&mut ffi);
            }
        }
    }
}
pub mod calyx_ffi_generated_top {
    use super::*;
    pub unsafe fn run_tests(ffi: &mut CalyxFFI) {
        for test in tests::calyx_ffi_generated_wrappers::CALYX_FFI_TESTS {
            test(ffi);
        }
    }
}
#[rustc_main]
#[coverage(off)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test_add, &test_sub])
}
