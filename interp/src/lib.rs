pub mod interpreter;
pub mod primitives;
pub mod serialization;
pub use utils::MemoryMap;
pub mod configuration;
pub mod debugger;
pub mod errors;
pub mod interpreter_ir;
pub mod logging;
mod macros;
mod structures;
mod tests;
mod utils;

pub mod flatten;

pub use structures::{environment, stk_env, values};
