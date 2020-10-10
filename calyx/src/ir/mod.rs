mod common;
mod component;
mod control;
/// Modules defining internal structures.
mod guard;

/// Re-export types at the module level.
pub use common::{RRC, WRC};
pub use component::{
    Assignment, Cell, CellType, Component, Direction, Group, Port,
    PortParent,
};
pub use control::{Control, Empty, Enable, If, Par, Seq, While};
pub use guard::Guard;

/// Module to transform AST programs into IR.
pub mod from_ast;
