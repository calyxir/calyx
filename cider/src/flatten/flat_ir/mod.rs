/// Core structures used by the rest of the IR
pub mod base;
/// Cell prototypes (i.e cell types)
pub mod cell_prototype;
/// Component definitions
pub mod component;
/// Module collecting the structures related to the control section of a Calyx
/// program.
pub mod control;
/// Module collecting the traits and structures related to the flattening
/// process. Internal and only used in the translator
pub(super) mod flatten_trait;
/// Module collecting structures around strings and identifiers
pub mod identifier;
/// Module collecting the structures related to the wires section of a Calyx program
pub mod wires;

/// Utility module with public imports for internal use
pub(crate) mod prelude {

    pub use super::base::*;
    pub use super::control::structures::*;
    pub use super::identifier::Identifier;
    pub use super::wires::core::*;
}

pub use control::translator::translate;
