use crate::backend::framework::Context;
use crate::lang::ast::{Component, Decl, Port, Std, Wire};
use crate::utils::*;
use std::collections::HashMap;

#[allow(unused)]
pub fn to_verilog(comp: &Component, c: &Context) -> String {
    format!(
        "
    // Component signature
    module {}
    (
        {}
    );
    // Wire declarations
    {}

    // Subcomponent Instances
    {}
    endmodule",
        comp.name,
        component_io(&comp),
        wire_declarations(&comp, &c),
        instances(&c)
    )
}

//==========================================
//        Component I/O Functions
//==========================================

/**
 * Returns a string with the list of all of a component's i/o pins
 */
#[allow(unused)]
pub fn component_io(c: &Component) -> String {
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
    combine(&inputs, ",\n        ", "")
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
        "".to_string()
    } else {
        format!("[{}:0] ", width - 1)
    }
}

//==========================================
//        Wire Declaration Functions
//==========================================
fn wire_declarations(comp: &Component, c: &Context) -> String {
    let wire_names = comp
        .get_wires()
        .into_iter()
        .map(|wire| wire_string(&wire, comp, c))
        .collect();

    combine(&[wire_names], "\n", "")
}

fn wire_string(wire: &Wire, comp: &Component, c: &Context) -> String {
    let width = Context::port_width(&wire.src, comp, c);
    format!("logic {}{};", bit_width(width), port_wire_id(&wire.src))
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

//==========================================
//        Subcomponent Instance Functions
//==========================================
// Intermediate data structures for string formatting
#[derive(Clone, Debug)]
pub struct RtlInst {
    pub comp_name: String,
    pub id: String,
    pub params: Vec<i64>,
    pub ports: HashMap<String, String>, // Maps Port names to wires
}

fn instances(c: &Context) -> String {
    let decls = c.toplevel.get_decl();
    let mut decls: Vec<RtlInst> = decls
        .into_iter()
        .map(|decl| component_to_inst(&decl, c))
        .collect();
    let prims = c.toplevel.get_std();
    let prims: Vec<RtlInst> = prims
        .into_iter()
        .map(|prim| prim_to_inst(&prim, c))
        .collect();
    decls.extend(prims);
    let strings: Vec<String> = decls.into_iter().map(inst_to_string).collect();
    combine(&strings, "\n", "")
}

fn component_to_inst(inst: &Decl, c: &Context) -> RtlInst {
    let comp = c.definitions.get(&inst.component).unwrap();
    let wires = c.toplevel.get_wires();
    let mut port_map: HashMap<String, String> = HashMap::new();
    for w in wires {
        if let Port::Comp { component, port } = &w.src {
            if component.clone() == inst.name {
                // Note that all port_wire_ids are currently based off the source
                port_map.insert(port.clone(), port_wire_id(&w.src));
            }
        }
        if let Port::Comp { component, port } = &w.dest {
            if component.clone() == inst.name {
                // Note that all port_wire_ids are currently based off the source
                port_map.insert(port.clone(), port_wire_id(&w.src));
            }
        }
    }
    RtlInst {
        comp_name: comp.name.clone(),
        id: inst.name.clone(),
        params: vec![],
        ports: port_map,
    }
}

fn prim_to_inst(inst: &Std, c: &Context) -> RtlInst {
    let prim = c.library.get(&inst.instance.name).unwrap();
    let wires = c.toplevel.get_wires();
    let mut port_map: HashMap<String, String> = HashMap::new();
    for w in wires {
        if let Port::Comp { component, port } = &w.src {
            if component.clone() == inst.name {
                // Note that all port_wire_ids are currently based off the source
                port_map.insert(port.clone(), port_wire_id(&w.src));
            }
        }
        if let Port::Comp { component, port } = &w.dest {
            if component.clone() == inst.name {
                // Note that all port_wire_ids are currently based off the source
                port_map.insert(port.clone(), port_wire_id(&w.src));
            }
        }
    }
    RtlInst {
        comp_name: prim.name.clone(),
        id: inst.name.clone(),
        params: inst.instance.params.clone(),
        ports: port_map,
    }
}

pub fn inst_to_string(inst: RtlInst) -> String {
    let ports: Vec<String> = inst
        .ports
        .iter()
        .map(|(port, wire)| format!(".{}({})", port, wire))
        .collect();
    let ports: String = combine(&ports, ",\n        ", "\n    ");
    let params: Vec<String> =
        inst.params.iter().map(|p| p.to_string()).collect();
    let params: String = combine(&params, ", ", "");
    format!(
        "{} #({}) {}\n    (\n        {});",
        inst.comp_name, params, inst.id, ports
    )
}
