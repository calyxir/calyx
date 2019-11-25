use crate::backend::framework::Context;
use crate::backend::rtl::templates;
use crate::lang::ast::{Component, Port, Wire};
use crate::utils;

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
        component_signature(&comp),
        wire_declarations(&comp, &c),
        ""
    )
}

fn component_signature(c: &Component) -> String {
    let pins = templates::comp_io(c);
    pins
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

    utils::combine(&wire_names, "\n", "")
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
        Port::Comp { component, port } => {
            return format!("{}_{}", component, port)
        }
        Port::This { port } => return port.clone(),
    }
}

fn bit_width(width: i64) -> String {
    if width < 1 {
        panic!("Invalid bit width!");
    } else if width == 1 {
        return format!("");
    } else {
        return format!("[{}:0] ", width - 1);
    }
}

//==========================================
//        Instance String Functions
//==========================================
fn component_instances() {}

fn primitive_instances() {}
