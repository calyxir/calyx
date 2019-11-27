use crate::lang::ast::*;
use crate::utils::*;
use std::collections::HashMap;

/**
 * This file contains definitions for intermediate RTL data structures
 * and how they get converted to strings
 */

// Intermediate data structures for string formatting
#[derive(Clone, Debug)]
pub struct RtlInst {
    pub comp_name: String,
    pub id: String,
    pub params: Vec<i64>,
    pub ports: HashMap<String, String>, // Maps Port names to wires
}

// Connections is a hashmap that maps src wires
// to the set of all of their destination wires
// This can then be used when instancing components
// to look up wire names
pub type Connections = HashMap<Port, Vec<Port>>;
// Environment type for all component definitions in
// scope. This includes all primitives and all components
// in the same namespace
pub type Components = HashMap<String, Component>;

#[allow(unused)]
pub fn component_string(
    comp: &Component,
    conn: &Connections,
    comps: &Components,
    insts: &[RtlInst],
) -> String {
    let io: String = comp_io(comp);
    let wires: String = wires(conn, comp, comps, insts);
    let inst_vec: Vec<String> =
        insts.iter().map(|inst| comp_inst(inst.clone())).collect();
    let inst_strings = combine(&inst_vec, "\n\n", "");
    format!(
        "
    // Component Name
    module {}
    (
        // Input/Output Ports
        {}
    );
    // Wire declarations
    {}

    // Subcomponent Instances
    {}

    endmodule
    ",
        comp.name, io, wires, inst_strings
    )
}

/**
 * Finds the port definition for a specified Port and Component
 * Returns None if the specified port is not in the provided component
 */
fn find_portdef(port: String, c: &Component) -> Option<Portdef> {
    let mut vec: Vec<Portdef> = c.inputs.clone();
    vec.append(&mut c.outputs.clone());

    for pd in vec {
        if pd.name == port {
            return Some(pd);
        }
    }
    None
}

/**
 * Looks up a component signature from an instance identifier. If component i1
 * is an instance of register, this will return the component signature of
 * the register component
 */
fn component_from_inst_id(
    id: Id,
    comps: &Components,
    insts: &[RtlInst],
) -> Option<Component> {
    let mut comp_name: Option<String> = None;
    for inst in insts {
        if id == inst.id {
            comp_name = Some(inst.comp_name.clone());
        }
    }

    match comp_name {
        Some(name) => Some(comps.clone().get(&name).unwrap().clone()),
        None => None,
    }
}

/**
 * Looks up bit width of a provided port based on the component it belongs to
 *
 * panics if it can't find the specified port
 */
pub fn port_width(
    p: &Port,
    top: &Component,
    comps: &Components,
    insts: &[RtlInst],
) -> i64 {
    match p {
        Port::Comp { component, port } => {
            let comp: Component =
                component_from_inst_id(component.clone(), comps, insts)
                    .unwrap();
            let pd: Portdef = find_portdef(port.clone(), &comp).unwrap();
            pd.width
        }
        Port::This { port } => {
            let pd: Portdef = find_portdef(port.clone(), top).unwrap();
            pd.width
        }
    }
}

/**
 * Create String that declares all of the wires for the Verilog output
 */
#[allow(unused)]
pub fn wires(
    conn: &Connections,
    top: &Component,
    comps: &Components,
    insts: &[RtlInst],
) -> String {
    let mut s = String::new();
    for p in conn.keys() {
        match p {
            Port::Comp { .. } => {
                let wire_name = port_wire_id(p);
                let bit_width = bit_width(port_width(p, top, comps, insts));
                let decl = format!("logic {}{};\n", bit_width, wire_name);
                s = format!("{}{}", s, decl); //Append decl to string
            }
            Port::This { .. } => {} // If Port is an input or output of toplevel, no need to declare wire
        }
    }
    s
}

#[allow(unused)]
pub fn comp_inst(inst: RtlInst) -> String {
    let ports: Vec<String> = inst
        .ports
        .iter()
        .map(|(port, wire)| format!(".{}({})", port, wire))
        .collect();
    let ports: String = combine(&ports, "\n,", "\n");
    let params: Vec<String> =
        inst.params.iter().map(|p| p.to_string()).collect();
    let params: String = combine(&params, ", ", "");
    format!("{}#({}) {}\n(\n{})", inst.comp_name, params, inst.id, ports)
}

/**
 * Returns a string with the list of all of a component's i/o pins
 */
#[allow(unused)]
pub fn comp_io(c: &Component) -> String {
    let inputs = c.inputs.clone();
    let mut inputs: Vec<String> = inputs
        .into_iter()
        .map(|pd| in_port(pd.width, pd.name))
        .collect();
    let outputs = c.outputs.clone();
    let mut outputs: Vec<String> = outputs
        .into_iter()
        .map(|pd| out_port(pd.width, pd.name))
        .collect();
    inputs.append(&mut outputs);
    combine(&inputs, ",\n", "")
}

pub fn in_port(width: i64, name: String) -> String {
    format!("input  logic {}{}", bit_width(width), name)
}

pub fn out_port(width: i64, name: String) -> String {
    format!("output logic {}{}", bit_width(width), name)
}

pub fn bit_width(width: i64) -> String {
    if width < 1 {
        panic!("Invalid bit width!");
    } else if width == 1 {
        format!("")
    } else {
        format!("[{}:0] ", width - 1)
    }
}

/**
 * Generates a string wirename for the provided Port object
 */
pub fn port_wire_id(p: &Port) -> String {
    match p {
        Port::Comp { component, port } => format!("{}_{}", component, port),
        Port::This { port } => port.clone(),
    }
}
