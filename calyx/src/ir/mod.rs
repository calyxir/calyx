//! Internal representation for the Calyx compiler.
//!
//! The representation is generated from the frontend AST.
//! The key differences between the frontend AST and the IR are:
//! 1. The IR uses pointers instead of [`Id`] to refer to things like [`Port`] and
//!    [`Group`].
//! 2. The IR attempts to represent similar concepts in a homogeneous manner.

// Modules defining internal structures.
mod attribute;
mod builder;
mod common;
mod component;
mod context;
mod control;
mod guard;
mod id;
mod primitives;
mod printer;
mod structure;

// Re-export types at the module level.
pub use attribute::{Attributes, GetAttributes};
pub use builder::Builder;
pub use common::{RRC, WRC};
pub use component::Component;
pub use context::{Context, LibrarySignatures};
pub use control::{Control, Empty, Enable, If, Invoke, Par, Seq, While};
pub use guard::Guard;
pub use id::Id;
pub use primitives::{PortDef, Primitive, Width};
pub use printer::IRPrinter;
pub use structure::{
    Assignment, Binding, Cell, CellIterator, CellType, CloneName, CombGroup,
    Direction, GetName, Group, Port, PortIterator, PortParent,
};

/// Visitor to traverse a control program.
pub mod traversal;

/// Module to transform AST programs into IR.
pub mod from_ast;

/// Convinience macros for constructing IR nodes.
mod macros;
