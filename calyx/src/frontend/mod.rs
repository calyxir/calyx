//! Frontend parsing and AST representation.
//!
//! Defines the frontend AST and the parser.
//! The frontend representation is transformed into the representation defined
//! in the `ir` module.

pub mod ast;
pub(super) mod parser;

pub use ast::NamespaceDef;
