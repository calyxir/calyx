pub mod attributes;
pub mod component;
pub mod control;
pub mod identifier;
pub mod wires;

pub(in super::flat_ir) mod prelude {

    pub use super::identifier::Identifier;
}
