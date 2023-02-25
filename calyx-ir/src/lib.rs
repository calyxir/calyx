//! Internal representation for the Calyx compiler.
//!
//! The representation is generated from the frontend AST.
//! The key differences between the frontend AST and the IR are:
//! 1. The IR uses pointers instead of [`Id`] to refer to things like [`Port`] and
//!    [`Group`].
//! 2. The IR attempts to represent similar concepts in a homogeneous manner.

// Modules defining internal structures.
mod builder;
mod common;
mod component;
mod context;
mod control;
mod guard;
mod printer;
mod reserved_names;
pub mod rewriter;
mod structure;

// Re-export types at the module level.
pub use builder::Builder;
pub use calyx_utils::{GetName, Id};
pub use common::{RRC, WRC};
pub use component::{Component, IdList};
pub use context::{BackendConf, Context, LibrarySignatures};
pub use control::{
    Cloner, Control, Empty, Enable, If, Invoke, Par, Seq, While,
};
pub use guard::{Guard, PortComp};
pub use printer::Printer;
pub use reserved_names::RESERVED_NAMES;
pub use rewriter::Rewriter;
pub use structure::{
    Assignment, Binding, Canonical, Cell, CellType, CombGroup, Group, Port,
    PortIterator, PortParent,
};

pub use calyx_frontend::{
    Attributes, Direction, GetAttributes, PortDef, Primitive, Width,
};

/// Module to transform AST programs into IR.
pub mod from_ast;

/// Convinience macros for constructing IR nodes.
mod macros;
