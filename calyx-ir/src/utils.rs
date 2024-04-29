//! Helpers used to examine calyx programs. Used in Xilinx and Yxi backends among others.
use super::{BoolAttr, Cell, Component, RRC};
#[cfg(feature = "yxi")]
use serde::Serialize;

// Returns Vec<String> of `@external` or `ref` memory names
pub fn external_and_ref_memories_names(comp: &Component) -> Vec<String> {
    external_and_ref_memories_cells(comp)
        .iter()
        .map(|cell_ref| cell_ref.borrow().name().to_string())
        .collect()
}

/// Gets all memory cells in top level marked `@external` or `ref`.
pub fn external_and_ref_memories_cells(comp: &Component) -> Vec<RRC<Cell>> {
    comp.cells
        .iter()
        // find external and ref memories
        .filter(|cell_ref| {
            let cell = cell_ref.borrow();
            cell.attributes.has(BoolAttr::External) || cell.is_reference()
        })
        .cloned()
        .collect()
}

#[cfg_attr(feature = "yxi", derive(Serialize))]
#[derive(Clone, Copy)]
pub enum MemoryType {
    Combinational,
    Sequential,
}

/// Parameters for std memories
pub struct MemInfo {
    pub memory_type: MemoryType,
    pub data_width: u64,
    pub dimensions: u64,
    //dimension sizes in order: d1, d2, etc.
    pub dimension_sizes: Vec<u64>,
    pub total_size: u64,
    //idx port width, in case size is ambiguous
    pub idx_sizes: Vec<u64>,
}

// Returns a vector of tuples containing memory info of [comp] of form:
// [(WIDTH, SIZE, IDX_SIZE)]
pub trait GetMemInfo {
    fn get_mem_info(&self) -> Vec<MemInfo>;
}

impl GetMemInfo for Vec<RRC<Cell>> {
    fn get_mem_info(&self) -> Vec<MemInfo> {
        self.iter()
              .map(|cr| {
                  let mem = cr.borrow();
                  let mut dimension_sizes: Vec<u64> = Vec::new();
                  let mut idx_sizes: Vec<u64> = Vec::new();
                  let dimensions: u64;
                  //let mem_cell_type = mem.prototype.get_name().unwrap(); //i.e. "comb_mem_d1"
                  let mem_type : MemoryType = if mem.is_comb_cell() {
                    MemoryType::Combinational
                  } else {
                    MemoryType::Sequential
                  };

                  match mem.prototype.get_name().unwrap().as_ref() {
                      "comb_mem_d1" | "seq_mem_d1" => {
                        dimension_sizes.push(mem.get_parameter("SIZE").unwrap());
                        dimensions = 1;
                      }
                      "comb_mem_d2" | "seq_mem_d2" => {
                        dimension_sizes.push(mem.get_parameter("D0_SIZE").unwrap());
                        dimension_sizes.push(mem.get_parameter("D1_SIZE").unwrap());
                        dimensions = 2;
                    }
                    "comb_mem_d3" | "seq_mem_d3" => {
                        dimension_sizes.push(mem.get_parameter("D0_SIZE").unwrap());
                        dimension_sizes.push(mem.get_parameter("D1_SIZE").unwrap());
                        dimension_sizes.push(mem.get_parameter("D2_SIZE").unwrap());
                        dimensions = 3;
                    }
                    "comb_mem_d4" | "seq_mem_d4" => {
                        dimension_sizes.push(mem.get_parameter("D0_SIZE").unwrap());
                        dimension_sizes.push(mem.get_parameter("D1_SIZE").unwrap());
                        dimension_sizes.push(mem.get_parameter("D2_SIZE").unwrap());
                        dimension_sizes.push(mem.get_parameter("D3_SIZE").unwrap());
                        dimensions = 4;
                      }
                      _ => {
                          panic!("cell `{}' marked with `@external' but is not a memory primitive.", mem.name())
                      }
                  };
                  if dimensions == 1 {
                    idx_sizes.push(mem.get_parameter("IDX_SIZE").unwrap());
                  } else {
                    for i in 0..dimensions {
                        idx_sizes.push(mem.get_parameter(format!("D{}_IDX_SIZE",i)).unwrap());
                    }
                  }
                  let total_size = dimension_sizes.clone().iter().product();
                  MemInfo {
                      memory_type: mem_type,
                      data_width: mem.get_parameter("WIDTH").unwrap(),
                      dimensions,
                      dimension_sizes,
                      total_size,
                      idx_sizes
                  }
              })
              .collect()
    }
}

impl GetMemInfo for Component {
    fn get_mem_info(&self) -> Vec<MemInfo> {
        external_and_ref_memories_cells(self).get_mem_info()
    }
}
