use calyx_ir::utils::MemoryType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ProgramInterface {
    pub toplevel: String,
    pub memories: Vec<Memory>,
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
