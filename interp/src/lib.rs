pub mod interpreter;
pub mod primitives;
pub use utils::MemoryMap;
mod configuration;

pub mod debugger;
pub mod errors;
pub mod interpreter_ir;
mod macros;
mod structures;

pub use structures::{environment, stk_env, values};

mod tests;
mod utils;

pub use configuration::SETTINGS;
