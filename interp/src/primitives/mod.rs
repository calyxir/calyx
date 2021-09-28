mod primitive;
pub use primitive::Entry;
pub use primitive::Primitive;
pub use primitive::Serializeable;

pub mod combinational;
pub(super) mod prim_utils;
pub mod stateful;
