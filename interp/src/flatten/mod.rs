pub mod flat_ir;
pub mod primitives;
pub mod structures;
pub(crate) mod text_utils;

mod setup;
pub use setup::{setup_simulation, setup_simulation_with_metadata};
