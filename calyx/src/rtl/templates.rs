use indoc::indoc;
use crate::utils::*;
use crate::lang::ast::*;
use std::collections::HashMap;


// Intermediate data structure conducive to string formatting
pub struct RtlInst {
    pub comp_name: String,
    pub id: String,
    pub params: Vec<i64>,
    pub ports: HashMap<String, String>,// Maps Port names to wires
}

pub fn comp () -> String {
    return indoc!("
    // Component Name
    module {}
    #(
        // Parameters
        {}
    )(
        // Input/Output Ports
        {}
    );
    // Wire declarations
    {}

    // Subcomponent Instances
    {}

    endmodule
    ").to_string();
}


pub fn comp_inst(inst: RtlInst) -> String {
    let ports: Vec<String> = inst.ports.iter().map(|(port, wire)| format!(".{}({})", port, wire)).collect();
    let ports: String = combine(&ports, "\n,", "\n");
    let params: Vec<String> = inst.params.iter().map(|p| p.to_string()).collect();
    let params: String = combine(&params, ", ", "");
    return format!("{}#({}) {}\n(\n{})", inst.comp_name, params, inst.id, ports);
}

pub fn in_port (width: i64, name: String) -> String {
    return format!("input  logic {}{}", bit_width(width), name); 
}

pub fn out_port (width: i64, name: String) -> String {
    return format!("output logic {}{}", bit_width(width), name); 
}

pub fn bit_width(width: i64) -> String {
    if width < 1 {
        panic!("Invalid bit width!");
    } 
    else if width == 1 {
        return format!("");
    } else {
        return format!("[{}:0] ", width-1);
    }
}

/**
 * Generates a string wirename for the provided Port object
 */
pub fn port_wire_id(p: Port) -> String {
    match p {
        Port::Comp {component, port} => return format!("{}_{}", component, port),
        Port::This {port} => return port,
    }
}