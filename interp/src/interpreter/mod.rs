//! The machinery for interpreting a Calyx program

mod component_interpreter;
mod control_interpreter;
mod group_interpreter;
mod interpreter_trait;
mod utils;

pub use component_interpreter::ComponentInterpreter;
pub use interpreter_trait::Interpreter;
pub use utils::{ConstCell, ConstPort};
