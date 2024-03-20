pub mod btor2_prim;
mod builder;
pub mod combinational;
pub(crate) mod macros;
pub mod prim_trait;
pub mod stateful;

pub(crate) use builder::build_primitive;
pub use prim_trait::Primitive;

use macros::*;
