//! Shared utilities.
mod global_sym;
pub mod math;
pub(crate) mod measure_time;
mod namegenerator;
mod out_file;
mod position;
mod weight_graph;

pub use global_sym::GSym;
pub use namegenerator::NameGenerator;
pub use out_file::OutputFile;
pub use position::{
    FileIdx, GPosIdx, GlobalPositionTable, PosIdx, PositionTable, WithPos,
};
pub use weight_graph::{BoolIdx, Idx, WeightGraph};
