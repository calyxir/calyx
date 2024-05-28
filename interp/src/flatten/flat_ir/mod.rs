pub mod attributes;
pub mod base;
pub mod cell_prototype;
pub mod component;
pub mod control;
pub mod flatten_trait;
pub mod identifier;
pub mod wires;

pub(crate) mod prelude {

    pub use super::base::*;
    pub use super::control::structures::*;
    pub use super::identifier::Identifier;
    pub use super::wires::core::*;
}

pub use control::translator::translate;
