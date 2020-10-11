//! Internal representation for the FuTIL compiler. The representation is
//! generated from the frontend AST.
//! The key differences between the frontend AST and the IR are:
//! 1. The IR uses pointers instead of Id's to refer to things like Ports and
//!    Groups.
//! 2. The IR attempts to represent similar concepts in a homogeneous manner.
mod common;
mod component;
mod control;
/// Modules defining internal structures.
mod guard;

/// Re-export types at the module level.
pub use common::{RRC, WRC};
pub use component::{
    Assignment, Cell, CellType, Component, Direction, Group, Port, PortParent,
};
pub use control::{Control, Empty, Enable, If, Par, Seq, While};
pub use guard::Guard;

/// Module to transform AST programs into IR.
pub mod from_ast;
