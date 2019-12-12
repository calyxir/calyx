use crate::backend::fsm::machine::{Edge, State, StateIndex, FSM};
use crate::utils::*;

//const reset_string: String = "reset".to_string();
//const clock_string: String = "clk".to_string();
//const state_string: String = "state".to_string();
//const next_state_string: String = "next_state".to_string();

#[allow(unused)]
fn to_verilog(fsm: &FSM) -> String {
    let module_name = "pls_change_me";
    let portdefs = "TODO\n";
    let wiredefs = format!("logic [{}:0] state, next_state;", fsm.state_bits());
    format!(
        "module {} (\n{});\n{}\n{}\n{}\n{}\n endmodule",
        module_name,
        portdefs,
        wiredefs,
        state_transition(fsm),
        next_state_logic(fsm),
        output_logic(fsm)
    )
}

fn state_transition(fsm: &FSM) -> String {
    format!(
        "always_ff @(posedge clk) begin
        if(reset)
            state <= {}'d0; // 0 default state?
        else
            state <= next_state;
    end",
        fsm.state_bits()
    )
}

fn next_state_logic(fsm: &FSM) -> String {
    let cases: Vec<String> = fsm
        .states
        .iter()
        .map(|(ind, st)| next_state_case(&st, fsm, ind))
        .collect();
    let cases = combine(&cases, "\n", "");
    format!(
        "always_comb begin\n    case (state)\n{}\n endcase\n end",
        cases
    )
}

// TODO:
// Set bitwidths of value? Currently hardcoded to 1 bit
// Need bitwidth of state
fn next_state_case(st: &State, fsm: &FSM, st_ind: &StateIndex) -> String {
    let if_statements: Vec<String> = st
        .transitions
        .iter()
        .map(|e| if_statement(&e, fsm))
        .collect();
    let if_statements = combine(&if_statements, "\n    else ", "");
    let else_statement = format!(
        "\n    else\n    next_state = {};",
        fsm.state_string(*st_ind)
    );
    format!(
        "{}: begin\n    {}{}\n    end",
        fsm.state_string(*st_ind),
        if_statements,
        else_statement
    )
}

// TODO:
// Set bitwidths of value? Currently hardcoded to 1 bit
// Need bitwidth of state
fn if_statement((inputs, st): &Edge, fsm: &FSM) -> String {
    let conditions: Vec<String> = inputs
        .iter()
        .map(|(_, id, value)| format!("{} == 1'd{}", id, value))
        .collect();
    let conditions: String = combine(&conditions, " && ", "");
    format!(
        "if ( {} )\n    next_state = {}'d{};",
        conditions,
        fsm.state_bits(),
        st.id + 1 //fsm.state_string(st)
    )
}

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
    let out_statements: Vec<String> = st
        .outputs
        .iter()
        .map(|(_, id, val)| format!("{} = 1'd{};", id, val))
        .collect();

    let out_statements = combine(&out_statements, "\n ", "");
    format!(
        "{}: begin\n    {}\n   end",
        fsm.state_string(*st_id),
        out_statements
    )
}
