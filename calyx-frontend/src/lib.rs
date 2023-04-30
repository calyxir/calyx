//! Frontend parsing and AST representation.
//!
//! Defines the frontend AST and the parser.
//! The frontend representation is transformed into the representation defined
//! in the `ir` module.

pub mod ast;
mod attributes;
mod common;
pub mod parser;
mod workspace;

pub use ast::NamespaceDef;
pub use attributes::{Attributes, GetAttributes};
pub use common::{Direction, PortDef, Primitive, Width};
pub use workspace::Workspace;
