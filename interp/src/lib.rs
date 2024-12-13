mod as_raw;
pub mod configuration;
pub mod debugger;
pub mod errors;
pub mod logging;
mod macros;
pub mod serialization;
mod tests;

pub mod flatten;

// ethan: griffin ok'd this
pub use baa::{BitVecOps, BitVecValue};
