//! This module contains all the modules defining the core IR structures used
//! by the rest of Cider and the conversion from the Calyx IR.

pub mod flat_ir;
pub mod primitives;
pub mod structures;
pub(crate) mod text_utils;

mod setup;
pub use setup::{setup_simulation, setup_simulation_with_metadata};
