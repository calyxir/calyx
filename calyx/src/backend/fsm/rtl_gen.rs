use super::machine::ValuedPort;
use crate::backend::fsm::machine::{Edge, State, StateIndex, FSM};
use crate::backend::rtl::gen;
use crate::lang::ast::{Component, Id};
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

#[allow(unused)]
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

fn input<'a>(id: Id) -> RcDoc<'a> {
    RcDoc::text(format!("input  logic {}", id))
}

fn output<'a>(id: Id) -> RcDoc<'a> {
    RcDoc::text(format!("output logic {}", id))
}

//==========================================
//        FSM State Transition Block
//==========================================
fn state_transition(fsm: &FSM) -> RcDoc<'_> {
    RcDoc::text("always_ff")
        .append(RcDoc::space())
        .append(RcDoc::text("@(posedge clk)"))
        .append(RcDoc::space())
        .append(RcDoc::text("begin"))
        .append(RcDoc::line().nest(4))
        .append(RcDoc::text("if ( reset )"))
        .append(RcDoc::line().nest(8))
        .append(RcDoc::text(format!(
            "state <= {}'d0; // 0 default state?",
            fsm.state_bits()
        )))
        .append(RcDoc::line().nest(4))
        .append(RcDoc::text("else"))
        .append(RcDoc::line().nest(8))
        .append(RcDoc::text("state <= next_state;"))
        .append(RcDoc::line())
        .append(RcDoc::text("end"))
}
//==========================================
//        FSM State Transition Logic
//==========================================
/// TODO add default case
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
        .append(RcDoc::text(fsm.state_bits().to_string()))
        .append(RcDoc::text("'d"))
        .append(RcDoc::text((st.id + 1).to_string()))
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
    //let out_statements = combine(&out_statements, "\n ", "");
    //format!(
    //    "{}: begin\n    {}\n   end",
    //    fsm.state_string(*st_id),
    //    out_statements
    //)
}

fn out_statement<'a>((_, port, value): ValuedPort) -> RcDoc<'a> {
    RcDoc::text(format!("{} = 1'd{};", port, value))
}
