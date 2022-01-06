pub mod interpreter;
pub mod primitives;
pub use utils::MemoryMap;
mod configuration;
pub mod debugger;
pub mod errors;
pub mod interpreter_ir;
pub mod logging;
mod macros;
mod structures;
mod tests;
mod utils;

pub use configuration::SETTINGS;
pub use structures::{environment, stk_env, values};
