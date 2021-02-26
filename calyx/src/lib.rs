//! # The Calyx Intermediate Language
//!
//! Calyx is an intermediate language for transforming high-level programs
//! into synthesizable hardware designs.
//! Calyx's key novelty is a split representation that captures the control-flow
//! and the structural detail of a hardware design.
//!
//! The control sub-language provides several high-level constructs: `while` (loops),
//! `if` (conditionals), `seq` (sequencing), `par` (parallel execution),
//! `invoke` (function calls).
//! These contructs simplify the process of encoding the control-flow of a
//! high-level program.
//!
//! Calyx's structural sub-language precisely capture details of the underlying
//! hardware. Structural programs specify guarded assignments [^1] using ports
//! in structural components.
//!
//! Take a look at the [language tutorial][lang-tut] for a complete overview.
//!
//!
//! [^1]: Calyx's guarded assignments are different from [Bluespec's rules][bsv-rules].
//! Rules can be dynamically aborted if there are conflicts at runtime and the
//! Bluespec compiler generates scheduling logic to detect such cases.
//! In contract, Calyx's schedule is defined using the control program and
//! requires no additional scheduling logic to detect aborts.
//!
//! [bsv-rules]: http://wiki.bluespec.com/Home/Rules
//! [lang-tut]: https://capra.cs.cornell.edu/docs/calyx/tutorial/language-tut.html
pub mod analysis;
pub mod backend;
pub mod errors;
pub mod frontend;
pub mod ir;
pub mod passes;
pub mod utils;
