//! # The Calyx Compiler
//!
//! This crate plumbs together the Calyx compiler crates and provides a command-line interface for the Calyx compiler.
//! What `clang` it to `llvm`, this crate is to the Calyx IL.
//! You SHOULD NOT depend on this crate since does things like installing the primitives library in a global location.
//! Instead, depend on the crates that this crate depends: [`calyx_frontend`], [`calyx_ir`], [`calyx_opt`].
pub mod cmdline;
