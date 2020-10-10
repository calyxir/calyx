/// Modules defining internal structures.
mod guard;
mod common;
mod component;
mod control;

/// Re-export types at the module level.
pub use common::{RRC, WRC};
pub use guard::Guard;
pub use component::{Port, Component, Cell, CellType, Direction, Group, Assignment};
pub use control::{Control, Empty, If, While, Enable, Seq, Par};

/// Module to transform AST programs into IR.
pub mod from_ast;
