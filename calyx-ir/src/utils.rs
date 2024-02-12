//! Helpers used to examine calyx programs. Used in Xilinx and Yxi backends among others.
use super::{BoolAttr, Cell, Component, RRC};
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
            cell.attributes.has(BoolAttr::External)
        })
        .cloned()
        .collect()
}

/// Parameters for std memories
pub struct MemInfo {
    pub width: u64,
    pub size: u64,
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
                  let mem_size: u64;
                  let mut idx_sizes: Vec<u64> = Vec::new();
                  let idx_count: u64;
                  match mem.prototype.get_name().unwrap().as_ref() {
                      "comb_mem_d1" | "seq_mem_d1" => {
                        mem_size = mem.get_parameter("SIZE").unwrap();
                        idx_count = 1;
                      }
                      "comb_mem_d2" | "seq_mem_d2" => {
                        mem_size = mem.get_parameter("D0_SIZE").unwrap()
                              * mem.get_parameter("D1_SIZE").unwrap();
                        idx_count = 2;
                    }
                    "comb_mem_d3" | "seq_mem_d3" => {
                        mem_size = mem.get_parameter("D0_SIZE").unwrap()
                        * mem.get_parameter("D1_SIZE").unwrap()
                        * mem.get_parameter("D2_SIZE").unwrap();
                        idx_count = 3;
                    }
                    "comb_mem_d4" | "seq_mem_d4" => {
                        mem_size = mem.get_parameter("D0_SIZE").unwrap()
                        * mem.get_parameter("D1_SIZE").unwrap()
                        * mem.get_parameter("D2_SIZE").unwrap()
                        * mem.get_parameter("D3_SIZE").unwrap();
                        idx_count = 4;
                      }
                      _ => {
                          panic!("cell `{}' marked with `@external' but is not a memory primitive.", mem.name())
                      }
                  };
                  if idx_count == 1 {
                    idx_sizes.push(mem.get_parameter("IDX_SIZE").unwrap());
                  } else {
                    for i in 1..idx_count {
                        idx_sizes.push(mem.get_parameter(format!("D{}_IDX_SIZE",i)).unwrap());
                    }
                  }
                  MemInfo {
                      width: mem.get_parameter("WIDTH").unwrap(),
                      size: mem_size,
                      idx_sizes
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
