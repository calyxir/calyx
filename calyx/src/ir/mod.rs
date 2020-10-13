//! Internal representation for the FuTIL compiler. The representation is
//! generated from the frontend AST.
//! The key differences between the frontend AST and the IR are:
//! 1. The IR uses pointers instead of `Id`s to refer to things like Ports and
//!    Groups.
//! 2. The IR attempts to represent similar concepts in a homogeneous manner.

mod builder;
/// Modules defining internal structures.
mod common;
mod component;
mod context;
mod control;
mod guard;
mod printer;
mod structure;

pub use builder::Builder;
/// Re-export types at the module level.
pub use common::{RRC, WRC};
pub use component::Component;
pub use context::Context;
pub use control::{Control, Empty, Enable, If, Par, Seq, While};
pub use guard::Guard;
pub use printer::IRPrinter;
pub use structure::{
    Assignment, Cell, CellType, Direction, Group, Port, PortParent,
};

/// Visitor to traverse a control program.
pub mod traversal;

/// Module to transform AST programs into IR.
pub mod from_ast;
