//! # The Calyx Compiler Driver
//!
//! This crate plumbs together the Calyx compiler crates and provides a command-line interface for the Calyx compiler.
//! What `clang` it to `llvm`, this crate is to the Calyx IL.
//!
//! For the most part, you don't want to directly rely on this crate and instead use the [`calyx_ir`] or the [`calyx_opt`] crates.
//! However, the [driver::run_compiler] function's is a good example for how to use the Calyx compiler crates.
pub mod backend;
pub mod cmdline;
pub mod driver;
