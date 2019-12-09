use super::machine::{Port, State, FSM};
use crate::lang::ast::{Component, Namespace, Portdef};

pub fn generate_fsms(syntax: &mut Namespace) -> Vec<FSM> {
    (&mut syntax.components)
        .into_iter()
        .filter_map(|comp| {
            if comp.name.starts_with("fsm_enable") {
                Some(enable_fsm(comp))
            } else {
                None
            }
        })
        .collect()
}

fn port_def_to_input(
    pre: &str,
    ports: Vec<Portdef>,
    comp_name: String,
) -> Vec<(Port, i64)> {
    ports
        .into_iter()
        .filter_map(|port: Portdef| {
            if port.name.starts_with(pre) {
                Some(((comp_name, port.name), 1))
            } else {
                None
            }
        })
        .collect()
}

pub fn enable_fsm(component: &Component) -> FSM {
    let mut start: State = State::empty();
    let mut mid: State = State::empty();
    let mut end: State = State::empty();

    // transitions
    let start_trans = (vec![((component.name.clone(), "valid"), 1)], mid);
    let mid_trans = (
        port_def_to_input(
            "ready",
            component.inputs.clone(),
            component.name.clone(),
        ),
        end,
    );

    // let outputs: Vec<(Port, i64)> = component
    //     .outputs
    //     .clone()

    // mid.outputs = outputs;

    // FSM::new(&start)

    let states = Box::new(vec![start, mid, end]);

    FSM {
        inputs: vec![],
        outputs: vec![],
        states,
        start: &states[0],
    }
}
