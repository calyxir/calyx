//! The FuTIL compiler frontend. Responsible for parser FuTIL programs and
//! FuTIL libraries.
//! The frontend representation is transformed into the representation defined
//! in the `ir` module.
pub mod ast;
pub mod parser;

pub use ast::NamespaceDef;
