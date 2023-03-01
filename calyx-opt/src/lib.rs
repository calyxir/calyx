//! # The Calyx Intermediate Language
//!
//! Calyx is an intermediate language for transforming high-level programs
//! into synthesizable hardware designs.
//! Calyx's key novelty is a split representation that captures the control-flow
//! and the structural detail of a hardware design.
//! Take a look at the [language tutorial][lang-tut] for a complete overview for the Calyx
//! intermediate langauge.
//!
//! This library defines the intermediate representation, i.e., the data structures used by the
//! compiler to analyze and transform programs.
//! The following example shows how to parse a Calyx program and generate the core data structure,
//! [ir::Context] which provides access to all the information in a program.
//!
//! ```rust
//! use std::io::Write;
//! use calyx_ir as ir;
//! use calyx_frontend as frontend;
//! use calyx_utils::CalyxResult;
//! fn main() -> CalyxResult<()> {
//!   // File to parse
//!   let file: std::path::PathBuf = "../tests/correctness/seq.futil".into();
//!   // Location of the calyx repository
//!   let lib_path: std::path::PathBuf = "../".into();
//!   // Parse the calyx program
//!   let ws = frontend::Workspace::construct(&Some(file), &lib_path)?;
//!   // Convert it into an ir::Context
//!   let mut ctx = ir::from_ast::ast_to_ir(ws)?;
//!   // Print out the components in the program
//!   let out = &mut std::io::stdout();
//!   for comp in &ctx.components {
//!       ir::Printer::write_component(comp, out)?;
//!       writeln!(out)?
//!   }
//!   Ok(())
//! }
//! ```
//!
//! [^1]: Calyx's guarded assignments are different from [Bluespec's rules][bsv-rules].
//! Rules can be dynamically aborted if there are conflicts at runtime and the
//! Bluespec compiler generates scheduling logic to detect such cases.
//! In contract, Calyx's schedule is defined using the control program and
//! requires no additional scheduling logic to detect aborts.
//!
//! [bsv-rules]: http://wiki.bluespec.com/Home/Rules
//! [lang-tut]: https://docs.calyxir.org/tutorial/language-tut.html
pub mod analysis;
pub mod default_passes;
pub mod pass_manager;
pub mod passes;
pub mod traversal;
