use crate::backend::fsm::machine::{Edge, State, StateIndex, FSM};
use crate::lang::ast::Id;
use crate::utils::*;
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
pub fn to_verilog(fsm: &FSM) -> String {
    let portdefs = "TODO\n";
    let wiredefs = format!("logic [{}:0] state, next_state;", fsm.state_bits());
    format!(
        "module {}\n\n{}\n\n{}\n\n{}\n\n{}\n endmodule",
        pretty_print(module_declaration(fsm)),
        wiredefs,
        pretty_print(state_transition(fsm)),
        pretty_print(next_state_logic(fsm)),
        output_logic(fsm)
    )
}

//==========================================
//        FSM Module Declaration Functions
//==========================================
fn module_declaration<'a>(fsm: &'a FSM) -> RcDoc<'a> {
    let module_name = "pls_change_me";
    let inputs = fsm.inputs().into_iter().map(|id| input(id));
    let outputs = fsm.outputs().into_iter().map(|id| output(id));
    RcDoc::text(format!("{} (", module_name))
        .append(RcDoc::line())
        .nest(4)
        .append(RcDoc::intersperse(
            inputs,
            RcDoc::text(",").append(RcDoc::line()).nest(4),
        ))
        .append(RcDoc::text(","))
        .append(RcDoc::line().nest(4))
        .append(RcDoc::intersperse(
            outputs,
            RcDoc::text(",").append(RcDoc::line()).nest(4),
        ))
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
fn output_logic(fsm: &FSM) -> String {
    let statements: Vec<String> = fsm
        .states
        .iter()
        .map(|(st_id, state)| output_state(&state, fsm, st_id))
        .collect();
    let statements = combine(&statements, "\n ", "");
    format!(
        "always_comb begin\n    case (state)\n{}\n endcase\n end",
        statements
    )

    //"TODO".to_string()
}

fn output_state(st: &State, fsm: &FSM, st_id: &StateIndex) -> String {
    let mut state_list: Vec<&String> = Vec::new();
    let mut out_statements: Vec<String> = st
        .outputs
        .iter()
        .map(|(_, id, val)| {
            state_list.push(id);
            format!("{} = 1'd{};", id, val)
        })
        .collect();

    let mut iter = state_list.iter();
    for outputs in &fsm.outputs {
        if !iter.any(|&x| x == outputs) {
            out_statements.push(format!("{} = 1'd0", outputs))
        }
    }

    let out_statements = combine(&out_statements, "\n ", "");
    format!(
        "{}: begin\n    {}\n   end",
        fsm.state_string(*st_id),
        out_statements
    )
}
