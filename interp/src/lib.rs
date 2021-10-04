pub mod environment;
pub mod interpreter;
pub mod primitives;
pub mod stk_env;
pub mod values;
pub use utils::MemoryMap;

pub mod debugger;
pub mod errors;
pub mod interpreter_ir;
mod macros;
mod ref_handler;
pub use ref_handler::RefHandler;
mod tests;
mod utils;
