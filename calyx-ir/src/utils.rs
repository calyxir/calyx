//! Helpers used to examine calyx programs. Used in Xilinx and Yxi backends among others.
use super::{BoolAttr, Cell, Component, RRC};
use calyx_utils::Id;
#[cfg(feature = "serialize")]
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

#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Clone, Copy)]
pub enum MemoryType {
    Combinational,
    Sequential,
    Dynamic,
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
        //Params of dimensions for multi dimensional memories. d1 memories use `"SIZE"`.
        let dimension_params = ["D0_SIZE", "D1_SIZE", "D2_SIZE", "D3_SIZE"];
        self.iter()
              .map(|cr| {
                  let mem = cr.borrow();
                  let mut dimension_sizes: Vec<u64> = Vec::new();
                  let mut idx_sizes: Vec<u64> = Vec::new();
                  let mem_cell_type = mem.prototype.get_name().unwrap(); //i.e. "comb_mem_d1"
                  let mem_type : MemoryType = if mem_cell_type.to_string().contains("comb") {
                      MemoryType::Combinational
                    } else if mem_cell_type.to_string().contains("seq") {
                        MemoryType::Sequential
                    } else {
                        MemoryType::Dynamic
                    };

                    let dimensions = dimension_count(mem_cell_type);
                    if dimensions == 1{
                        dimension_sizes.push(mem.get_parameter("SIZE").unwrap());
                        idx_sizes.push(mem.get_parameter("IDX_SIZE").unwrap());
                    }
                    else if dimensions > 1 && dimensions <= 4{
                        for i in 0..dimensions {
                            dimension_sizes.push(mem.get_parameter(dimension_params[i as usize]).unwrap());
                            idx_sizes.push(mem.get_parameter(format!("D{}_IDX_SIZE",i)).unwrap());
                        }
                    }
                    else{
                            unreachable!("It is not expected for memory primitives to have more than 4 dimensions.");
                    };
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

fn dimension_count(mem_id: Id) -> u64 {
    let mem_name = mem_id.as_ref();

    if mem_name.contains("d1") {
        1
    } else if mem_name.contains("d2") {
        2
    } else if mem_name.contains("d3") {
        3
    } else if mem_name.contains("d4") {
        4
    } else {
        panic!("Cell {} does not seem to be a memory primitive. Memory primitives are expected to have 1-4 dimensions inclusive.", mem_name);
    }
}
