mod primitive_traits;

pub use primitive_traits::Named;
pub use primitive_traits::Primitive;

pub mod combinational;
pub(super) mod prim_utils;
pub mod stateful;
