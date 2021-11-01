pub mod environment;
pub mod interpreter;
pub mod primitives;
pub mod stk_env;
pub mod values;
pub use utils::MemoryMap;
mod configuration;

pub mod debugger;
pub mod errors;
pub mod interpreter_ir;
mod macros;
mod tests;
mod utils;

pub use configuration::SETTINGS;
