//! Verilog-level names for the primitives

/// Register
pub const STD_REG: &str = "std_reg";

// Memories
pub const STD_MEM_D1: &str = "std_mem_d1";
pub const STD_MEM_D2: &str = "std_mem_d2";
pub const STD_MEM_D3: &str = "std_mem_d3";
pub const STD_MEM_D4: &str = "std_mem_d4";

// Sequential Memories
pub const SEQ_MEM_D1: &str = "seq_mem_d1";
pub const SEQ_MEM_D2: &str = "seq_mem_d2";
pub const SEQ_MEM_D3: &str = "seq_mem_d3";
pub const SEQ_MEM_D4: &str = "seq_mem_d4";

/// The primitive supports loading input value from a file using `readmemh` and
/// `writememh` calls in Verilog.
pub fn is_loadable(type_name: &str) -> bool {
    matches!(
        type_name,
        STD_REG
            | STD_MEM_D1
            | STD_MEM_D2
            | STD_MEM_D3
            | STD_MEM_D4
            | SEQ_MEM_D1
            | SEQ_MEM_D2
            | SEQ_MEM_D3
            | SEQ_MEM_D4
    )
}

/// Verilog module path to load values into.
pub fn load_path(type_name: &str) -> &str {
    match type_name {
        STD_REG => "mem",
        STD_MEM_D1 | STD_MEM_D2 | STD_MEM_D3 | STD_MEM_D4 => "mem",
        SEQ_MEM_D1 => "mem",
        SEQ_MEM_D2 | SEQ_MEM_D3 | SEQ_MEM_D4 => "mem.mem",
        _ => unreachable!("Unknown loadable primitive: {type_name}"),
    }
}
