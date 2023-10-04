//! Shared utilities for the Calyx compiler.
mod errors;
mod id;
mod namegenerator;
mod out_file;
mod position;
mod weight_graph;
mod mem_helpers;

mod math;
pub(crate) mod measure_time;

pub use errors::{CalyxResult, Error};
pub use id::{GSym, GetName, Id};
pub use math::bits_needed_for;
pub use namegenerator::NameGenerator;
pub use out_file::OutputFile;
pub use position::{
    FileIdx, GPosIdx, GlobalPositionTable, PosIdx, PositionTable, WithPos,
};
pub use weight_graph::{BoolIdx, Idx, WeightGraph};
pub use mem_helpers::{external_memories_names, external_memories_cells, get_mem_info, MemInfo};
