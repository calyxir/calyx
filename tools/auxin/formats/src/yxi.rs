use serde::{Deserialize, Serialize};

use crate::json_common;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum MemoryType {
    Combinational,
    Sequential,
    Dynamic,
}

#[derive(Serialize)]
pub struct ProgramInterface {
    toplevel: String,
    memories: Vec<Memory>,
}

#[derive(Serialize, Deserialize)]
pub struct Memory {
    pub name: String,
    pub memory_type: MemoryType,
    pub data_width: u64,
    pub dimensions: u64,
    pub dimension_sizes: Vec<u64>,
    pub total_size: u64, //number of cells in memory
    pub idx_sizes: Vec<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct MemInfo {
    pub info: Memory,
    pub format: json_common::FormatInfo,
}
