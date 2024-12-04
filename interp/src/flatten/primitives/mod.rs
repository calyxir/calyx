mod builder;
pub mod combinational;
pub(crate) mod macros;
pub mod prim_trait;
pub mod stateful;
pub mod utils;

pub(crate) use builder::build_primitive;
pub use prim_trait::Primitive;

use macros::*;
