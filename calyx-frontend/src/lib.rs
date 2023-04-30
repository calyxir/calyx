//! Frontend parsing and AST representation.
//!
//! Defines the frontend AST and the parser.
//! The frontend representation is transformed into the representation defined
//! in the `ir` module.

pub mod ast;
pub mod parser;

mod attribute;
mod attributes;
mod common;
mod workspace;

pub use ast::NamespaceDef;
pub use attribute::{Attribute, DEPRECATED_ATTRIBUTES};
pub use attributes::{Attributes, GetAttributes};
pub use common::{Direction, PortDef, Primitive, Width};
pub use workspace::Workspace;
