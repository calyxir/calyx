pub use super::{
    value_from_u64, CalyxFFI, CalyxFFIComponent, CalyxFFIComponentRef, Value,
};
pub use calyx_ffi_macro::{calyx_ffi, calyx_ffi_test, calyx_ffi_tests};
pub use calyx_ir;
pub use interp;
pub use paste;

#[macro_export]
macro_rules! declare_interface {
    ($name:ident($($input:ident: $input_width:literal),*)
    -> ($($output:ident: $output_width:literal),*)
    $(impl {
        $(fn $fn:ident(&mut $self:ident $(, $arg:ident: $argty:ty)* $(,)?) $(-> $ret:ty)? $body:block)*
    })? ) => {
        calyx_ffi::prelude::paste::paste! {
            pub trait $name: CalyxFFIComponent {
                $(
                    fn [<$input _bits>](&mut self) -> &mut calyx_ffi::Value<$input_width>;

                    fn [<set_ $input>](&mut self, value: u64);
                )*
                $(
                    fn [<$output _bits>](&self) -> &calyx_ffi::Value<$output_width>;

                    fn $output(&self) -> u64;
                )*
                $($(
                    fn $fn(&mut $self, $($arg: $argty),*) $(-> $ret)* {$body}
                )*)*
            }
        }
    };
}
