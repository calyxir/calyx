use super::machine::ValuedPort;
use crate::backend::fsm::machine::{Edge, State, StateIndex, FSM};
use crate::backend::rtl::gen;
use crate::lang::ast::{Component, Id, Portdef};
use pretty::RcDoc;

//const reset_string: String = "reset".to_string();
//const clock_string: String = "clk".to_string();
//const state_string: String = "state".to_string();
//const next_state_string: String = "next_state".to_string();
fn pretty_print(doc: RcDoc) -> String {
    let mut w = Vec::new();
    doc.render(80, &mut w).unwrap();
    String::from_utf8(w).unwrap()
}

pub fn to_verilog(fsm: &FSM, component: &Component) -> String {
    let wiredefs =
        format!("logic [{}:0] state, next_state;", fsm.state_bits() - 1);
    let doc = RcDoc::text("module")
        .append(RcDoc::space())
        .append(module_declaration(component))
        .append(RcDoc::line())
        .append(RcDoc::text(wiredefs))
        .append(RcDoc::line())
        .append(state_transition(fsm))
        .append(RcDoc::line())
        .append(next_state_logic(fsm))
        .append(RcDoc::line())
        .append(output_logic(fsm))
        .append(RcDoc::line())
        .append(RcDoc::text("endmodule"))
        .append(RcDoc::hardline());
    pretty_print(doc)
}

pub fn data_lut_verilog(component: &Component) -> String {
    let doc = RcDoc::text("module")
        .append(RcDoc::space())
        .append(module_declaration(component))
        .append(RcDoc::line())
        .append(data_lut_switch(&component))
        .append(RcDoc::line())
        .append(RcDoc::text("endmodule"))
        .append(RcDoc::hardline());
    pretty_print(doc)
}

fn data_lut_switch(component: &Component) -> RcDoc {
    let (head, tail) = component.inputs.split_at(component.inputs.len() / 2);
    let cases = head.iter().enumerate().map(|(idx, h)| {
        data_lut_case(
            tail.len(),
            idx,
            &component.outputs[0].name,
            h.name.clone(),
        )
    });
    RcDoc::text("assign")
        .append(RcDoc::space())
        .append(RcDoc::text(&component.outputs[1].name))
        .append(RcDoc::space())
        .append(RcDoc::text("="))
        .append(RcDoc::space())
        .append(RcDoc::intersperse(
            tail.iter().map(|t| RcDoc::text(t.name.clone())),
            RcDoc::text(" | "),
        ))
        .append(RcDoc::text(";"))
        .append(RcDoc::line())
        .append(RcDoc::text("always_comb"))
        .append(RcDoc::space())
        .append(RcDoc::text("begin"))
        .append(RcDoc::line().nest(4))
        .append(RcDoc::text("case"))
        .append(RcDoc::space())
        .append(RcDoc::text("({"))
        .append(portdef_to_doc(&tail))
        .append(RcDoc::text("})"))
        .append(
            RcDoc::line()
                .nest(4)
                .append(
                    RcDoc::intersperse(cases, RcDoc::line().nest(4))
                        .append(RcDoc::line()),
                )
                .nest(4)
                .append(RcDoc::text("default: "))
                .append(RcDoc::text(&component.outputs[0].name))
                .append(RcDoc::text("= 0;"))
                .append(RcDoc::line())
                .nest(4),
        )
        .append(RcDoc::text("endcase"))
        .append(RcDoc::line())
        .append(RcDoc::text("end"))
}

fn data_lut_case<'a>(
    num: usize,
    idx: usize,
    name: &'a str,
    out: String,
) -> RcDoc<'a> {
    let mut bits = format!("{}'b", num);
    for i in 0..num {
        if i == idx {
            bits.push('1')
        } else {
            bits.push('0')
        }
    }
    RcDoc::text(bits)
        .append(RcDoc::text(":"))
        .append(RcDoc::space())
        .append(RcDoc::text(name))
        .append(RcDoc::space())
        .append(RcDoc::text("= "))
        .append(RcDoc::text(out))
        .append(RcDoc::text(";"))
}

pub fn control_lut_verilog(component: &Component) -> String {
    let doc = RcDoc::text("module")
        .append(RcDoc::space())
        .append(module_declaration(component))
        .append(RcDoc::line())
        .append(control_lut_switch(&component))
        .append(RcDoc::line())
        .append(RcDoc::text("endmodule"))
        .append(RcDoc::hardline());
    pretty_print(doc)
}

fn control_lut_switch(component: &Component) -> RcDoc {
    let cases = component.inputs.iter().enumerate().map(|(idx, _)| {
        control_lut_case(
            component.inputs.len(),
            idx,
            &component.outputs[0].name,
        )
    });
    RcDoc::text("always_comb")
        .append(RcDoc::space())
        .append(RcDoc::text("begin"))
        .append(RcDoc::line().nest(4))
        .append(RcDoc::text("case"))
        .append(RcDoc::space())
        .append(RcDoc::text("({"))
        .append(portdef_to_doc(&component.inputs))
        .append(RcDoc::text("})"))
        .append(
            RcDoc::line()
                .nest(4)
                .append(
                    RcDoc::intersperse(cases, RcDoc::line().nest(4))
                        .append(RcDoc::line()),
                )
                .nest(4)
                .append(RcDoc::text("default: "))
                .append(RcDoc::text(&component.outputs[0].name))
                .append(RcDoc::text("= 0;"))
                .append(RcDoc::line())
                .nest(4),
        )
        .append(RcDoc::text("endcase"))
        .append(RcDoc::line())
        .append(RcDoc::text("end"))
}

fn portdef_to_doc<'a>(ports: &[Portdef]) -> RcDoc<'a> {
    let port_docs = ports.iter().map(|p| RcDoc::text(p.name.clone()));
    RcDoc::intersperse(port_docs, RcDoc::text(", "))
}

fn control_lut_case<'a>(num: usize, idx: usize, name: &'a str) -> RcDoc<'a> {
    let mut bits = format!("{}'b", num);
    for i in 0..num {
        if i == idx {
            bits.push('1')
        } else {
            bits.push('0')
        }
    }
    RcDoc::text(bits)
        .append(RcDoc::text(":"))
        .append(RcDoc::space())
        .append(RcDoc::text(name))
        .append(RcDoc::space())
        .append(RcDoc::text("= 1;"))
}

//==========================================
//        FSM Module Declaration Functions
//==========================================
fn module_declaration(comp: &Component) -> RcDoc<'_> {
    let module_name = &comp.name;
    RcDoc::text(format!("{} (", module_name))
        .append(gen::component_io(&comp).nest(4))
        .append(RcDoc::line())
        .append(RcDoc::text(");"))
}

// fn input<'a>(id: Id) -> RcDoc<'a> {
//     RcDoc::text(format!("input  logic {}", id))
// }

// fn output<'a>(id: Id) -> RcDoc<'a> {
//     RcDoc::text(format!("output logic {}", id))
// }

//==========================================
//        FSM State Transition Block
//==========================================
fn state_transition(_fsm: &FSM) -> RcDoc<'_> {
    RcDoc::text("always_ff")
        .append(RcDoc::space())
        .append(RcDoc::text("@(posedge clk)"))
        .append(RcDoc::space())
        .append(RcDoc::text("begin"))
        .append(RcDoc::line().nest(4))
        // .append(RcDoc::text("if ( !valid )"))
        // .append(RcDoc::line().nest(8))
        // .append(RcDoc::text(format!(
        //     "state <= {}'d0; // 0 default state?",
        //     fsm.state_bits()
        // )))
        // .append(RcDoc::line().nest(4))
        // .append(RcDoc::text("else"))
        // .append(RcDoc::line().nest(8))
        .append(RcDoc::text("state <= next_state;"))
        .append(RcDoc::line())
        .append(RcDoc::text("end"))
}
//==========================================
//        FSM State Transition Logic
//==========================================
fn next_state_logic(fsm: &FSM) -> RcDoc<'_> {
    let cases = fsm
        .states
        .iter()
        .map(|(ind, st)| next_state_case(&st, fsm, ind));
    RcDoc::text("always_comb")
        .append(RcDoc::space())
        .append(RcDoc::text("begin"))
        .append(RcDoc::line().nest(4))
        .append(RcDoc::text("case"))
        .append(RcDoc::space())
        .append(RcDoc::text("(state)"))
        .append(
            RcDoc::line()
                .nest(4)
                .append(
                    RcDoc::intersperse(cases, RcDoc::line().nest(4))
                        .append(RcDoc::line()),
                )
                .append(RcDoc::line())
                .nest(4)
                .append(RcDoc::text("default: "))
                .append(RcDoc::line())
                .nest(4)
                .append(RcDoc::text(format!(
                    "next_state = {}'d0;",
                    fsm.state_bits()
                )))
                .append(RcDoc::line())
                .nest(4),
        )
        .append(RcDoc::text("endcase"))
        .append(RcDoc::line())
        .append(RcDoc::text("end"))
}

// TODO:
// Set bitwidths of value? Currently hardcoded to 1 bit
// Need bitwidth of state
fn next_state_case<'a>(
    st: &'a State,
    fsm: &'a FSM,
    st_ind: &'a StateIndex,
) -> RcDoc<'a> {
    let if_statements = st.transitions.iter().map(|e| if_statement(&e, fsm));
    RcDoc::text(fsm.state_string(*st_ind))
        .append(RcDoc::text(":"))
        .append(RcDoc::space())
        .append(RcDoc::text("begin"))
        .append(
            (RcDoc::line()
                .nest(4)
                .append(RcDoc::intersperse(
                    if_statements,
                    RcDoc::line()
                        .append(RcDoc::text("else"))
                        .append(RcDoc::space())
                        .nest(4),
                ))
                .append(RcDoc::line().nest(4))
                .append(RcDoc::text("else"))
                .append(RcDoc::line().nest(8))
                .append(RcDoc::text("next_state"))
                .append(RcDoc::space())
                .append(RcDoc::text("="))
                .append(RcDoc::space())
                .append(RcDoc::text(fsm.state_string(*st_ind)))
                .append(RcDoc::text(";")))
            .append(RcDoc::line()),
        )
        .nest(4)
        .append(RcDoc::text("end"))
}

// TODO:
// Set bitwidths of value? Currently hardcoded to 1 bit
// Need bitwidth of state
fn if_statement<'a>((inputs, st): &'a Edge, fsm: &'a FSM) -> RcDoc<'a> {
    let conditions = inputs.iter().map(condition);
    //let conditions: String = combine(&conditions, " && ", "");
    RcDoc::text("if")
        .append(RcDoc::space())
        .append(RcDoc::text("("))
        .append(RcDoc::space())
        .append(RcDoc::intersperse(
            conditions,
            RcDoc::line()
                .append(RcDoc::text("&&"))
                .append(RcDoc::space())
                .nest(4)
                .group(),
        ))
        .append(RcDoc::space())
        .append(RcDoc::text(")"))
        .append(RcDoc::line().nest(8))
        .append(RcDoc::text("next_state"))
        .append(RcDoc::space())
        .append(RcDoc::text("="))
        .append(RcDoc::space())
        .append(RcDoc::text(fsm.state_string(*st)))
        .append(RcDoc::text(";"))
}

/// Verilog string for a single condition in an if-statement
fn condition((_, id, value): &(String, String, i64)) -> RcDoc<'_> {
    RcDoc::text(id)
        .append(RcDoc::space())
        .append(RcDoc::text("=="))
        .append(RcDoc::space())
        .append(RcDoc::text("1'd"))
        .append(RcDoc::text(value.to_string()))
}

//==========================================
//        FSM State Output Logic
//==========================================
fn output_logic(fsm: &FSM) -> RcDoc<'_> {
    let statements = fsm
        .states
        .iter()
        .map(|(st_id, state)| state_outputs(&state, fsm, st_id));
    RcDoc::text("always_comb")
        .append(RcDoc::space())
        .append(RcDoc::text("begin"))
        .append(RcDoc::line().nest(4))
        .append(RcDoc::text("case"))
        .append(RcDoc::space())
        .append(RcDoc::text("(state)"))
        .append(
            RcDoc::line()
                .nest(4)
                .append(
                    RcDoc::intersperse(statements, RcDoc::line().nest(4))
                        .append(RcDoc::line()),
                )
                .append(RcDoc::line().nest(4))
                .append(default_outputs(fsm))
                .append(RcDoc::line().nest(4))
                .nest(4),
        )
        .append(RcDoc::text("endcase"))
        .append(RcDoc::line())
        .append(RcDoc::text("end"))

    //"TODO".to_string()
}

fn has_port(p: &Id, v: &[ValuedPort]) -> bool {
    for (_, port, _) in v {
        if port == p {
            return true;
        }
    }
    false
}

fn state_outputs<'a>(
    st: &'a State,
    fsm: &'a FSM,
    st_id: &'a StateIndex,
) -> RcDoc<'a> {
    let mut outputs: Vec<ValuedPort> = st.outputs.clone();
    for port in fsm.outputs() {
        if !has_port(&port, &outputs) {
            outputs.push(("".to_string(), port, 0));
        }
    }
    let outputs = outputs.into_iter().map(out_statement);
    RcDoc::text(fsm.state_string(*st_id))
        .append(RcDoc::text(":"))
        .append(RcDoc::space())
        .append(RcDoc::text("begin"))
        .append(
            RcDoc::line()
                .nest(4)
                .append(RcDoc::intersperse(outputs, RcDoc::line().nest(4))),
        )
        .append(RcDoc::line())
        .nest(4)
        .append(RcDoc::text("end"))
}

fn default_outputs<'a>(fsm: &'a FSM) -> RcDoc<'a> {
    let mut outputs: Vec<ValuedPort> = Vec::new();
    for port in fsm.outputs() {
        if !has_port(&port, &outputs) {
            outputs.push(("".to_string(), port, 0));
        }
    }
    let outputs = outputs.into_iter().map(out_statement);
    RcDoc::text("default")
        .append(RcDoc::text(":"))
        .append(RcDoc::space())
        .append(RcDoc::text("begin"))
        .append(
            RcDoc::line()
                .nest(4)
                .append(RcDoc::intersperse(outputs, RcDoc::line().nest(4))),
        )
        .append(RcDoc::line())
        .nest(4)
        .append(RcDoc::text("end"))
}

fn out_statement<'a>((_, port, value): ValuedPort) -> RcDoc<'a> {
    RcDoc::text(format!("{} = 1'd{};", port, value))
}
