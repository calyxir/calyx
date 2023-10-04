//! Helpers used to examine calyx programs. Used in Xilinx and Yxi backends among others.
use calyx_ir as ir;
// Returns Vec<String> of memory names
pub fn external_memories_names(comp: &ir::Component) -> Vec<String> {
  external_memories_cells(comp)
      .iter()
      .map(|cell_ref| cell_ref.borrow().name().to_string())
      .collect()
}

// Gets all memory cells in top level marked external.
// Common use is to get memories marked
pub fn external_memories_cells(comp: &ir::Component) -> Vec<ir::RRC<ir::Cell>> {
  comp.cells
      .iter()
      // find external memories
      .filter(|cell_ref| {
          let cell = cell_ref.borrow();
          cell.attributes.has(ir::BoolAttr::External)
      })
      .cloned()
      .collect()
}


/// Parameters for std memories
pub struct MemInfo {
  pub width: u64,
  pub size: u64,
  //idx port width, in case size is ambiguous
  pub idx_size: u64,
}

// Returns a vector of tuples containing external memory info of [comp] of form:
// [(WIDTH, SIZE, IDX_SIZE)]
pub fn get_mem_info(comp: &ir::Component) -> Vec<MemInfo> {
  external_memories_cells(comp)
      .iter()
      .map(|cr| {
          let mem = cr.borrow();
          let mem_size = match mem.prototype.get_name().unwrap().as_ref() {
              "std_mem_d1" | "seq_mem_d1" => {
                  mem.get_parameter("SIZE").unwrap()
              }
              "std_mem_d2" | "seq_mem_d2" => {
                  mem.get_parameter("D0_SIZE").unwrap()
                      * mem.get_parameter("D1_SIZE").unwrap()
              }
              "std_mem_d3" | "seq_mem_d3" => {
                  mem.get_parameter("D0_SIZE").unwrap()
                      * mem.get_parameter("D1_SIZE").unwrap()
                      * mem.get_parameter("D2_SIZE").unwrap()
              }

              "std_mem_d4" | "seq_mem_d4" => {
                  mem.get_parameter("D0_SIZE").unwrap()
                      * mem.get_parameter("D1_SIZE").unwrap()
                      * mem.get_parameter("D2_SIZE").unwrap()
                      * mem.get_parameter("D3_SIZE").unwrap()
              }
              _ => {
                  panic!("cell `{}' marked with `@external' but is not a memory primitive.", mem.name())
              }
          };

          MemInfo {
              width: mem.get_parameter("WIDTH").unwrap(),
              size: mem_size,
              idx_size: mem.get_parameter("IDX_SIZE").unwrap(),
          }
      })
      .collect()
}
