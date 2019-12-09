use super::machine::{ValuedPort, FSM};
use crate::lang::ast::{Component, Namespace, Portdef};

pub fn generate_fsms(syntax: &mut Namespace) -> Vec<FSM> {
    (&mut syntax.components)
        .iter_mut()
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
) -> Vec<ValuedPort> {
    ports
        .into_iter()
        .filter_map(|port: Portdef| {
            if port.name.starts_with(pre) {
                Some((comp_name.clone(), port.name, 1))
            } else {
                None
            }
        })
        .collect()
}

pub fn enable_fsm(component: &Component) -> FSM {
    let (start, mut fsm) = FSM::new();

    let mid = fsm.new_state();
    let end = fsm.new_state();

    // transitions
    fsm.get_state(start).add_transition((
        vec![(component.name.clone(), "valid".to_string(), 1)],
        mid,
    ));
    fsm.get_state(mid).add_transition((
        port_def_to_input(
            "ready",
            component.inputs.clone(),
            component.name.clone(),
        ),
        end,
    ));
    fsm.get_state(end).add_transition((
        vec![(component.name.clone(), "reset".to_string(), 1)],
        start,
    ));

    // outputs
    fsm.get_state(mid).add_outputs(&mut port_def_to_input(
        "valid",
        component.outputs.clone(),
        component.name.clone(),
    ));
    fsm.get_state(end).push_output((
        component.name.clone(),
        "ready".to_string(),
        1,
    ));

    fsm
}
