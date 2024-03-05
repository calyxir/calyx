//! Helpers used to examine calyx programs. Used in Xilinx and Yxi backends among others.
use super::{BoolAttr, Cell, Component, RRC};
use serde::Serialize;

// Returns Vec<String> of memory names
pub fn external_memories_names(comp: &Component) -> Vec<String> {
    external_memories_cells(comp)
    .iter()
    .map(|cell_ref| cell_ref.borrow().name().to_string())
    .collect()
}

// Gets all memory cells in top level marked external.
pub fn external_memories_cells(comp: &Component) -> Vec<RRC<Cell>> {
    comp.cells
    .iter()
    // find external memories
    .filter(|cell_ref| {
        let cell = cell_ref.borrow();
        cell.attributes.has(BoolAttr::External) || cell.is_reference()
    })
    .cloned()
    .collect()
}

#[cfg_attr(feature="serialize", derive(Serialize))]
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
    //idx port width, in case size is ambiguous
    pub idx_sizes: Vec<u64>,
}

// Returns a vector of tuples containing external memory info of [comp] of form:
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

                //   if mem_cell_type.as_ref().starts_with("comb") {
                //     MemoryType::Combinational
                //   } else if mem_cell_type.as_ref().starts_with("seq"){
                //     MemoryType::Sequential
                //   } else {
                //     panic!("cell `{}' is neither a combinational nor sequential memory.", mem.name());
                //   };

                //   match mem_cell_type[0 .. 4].as_ref() {
                //     "comb" => MemoryType::Combinational,
                //     "seq" => MemoryType::Sequential,
                //     _ => panic!("cell `{}' is neither a combinational nor sequential memory.", mem.name())
                //   };
                

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
                    for i in 1..dimensions {
                        idx_sizes.push(mem.get_parameter(format!("D{}_IDX_SIZE",i)).unwrap());
                    }
                  }
                  MemInfo {
                      memory_type: mem_type,
                      data_width: mem.get_parameter("WIDTH").unwrap(),
                      dimensions: dimensions,
                      dimension_sizes: dimension_sizes,
                      idx_sizes : idx_sizes
                  }
              })
              .collect()
    }
}

impl GetMemInfo for Component {
    fn get_mem_info(&self) -> Vec<MemInfo> {
        external_memories_cells(self).get_mem_info()
    }
}
