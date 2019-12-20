use super::machine::{ValuedPort, FSM};
use crate::lang::ast::{Component, Portdef};

pub fn generate_fsm(comp: &Component) -> Option<FSM> {
    if comp.name.starts_with("fsm_enable") {
        Some(enable_fsm(comp))
    } else if comp.name.starts_with("fsm_par") {
        Some(par_fsm(comp))
    } else if comp.name.starts_with("fsm_seq") {
        Some(seq_fsm(comp))
    } else if comp.name.starts_with("fsm_if") {
        Some(if_fsm(comp))
    } else if comp.name.starts_with("fsm_while") {
        Some(while_fsm(comp))
    } else {
        None
    }
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
    let (start, mut fsm) = FSM::new(&component.name);

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
        vec![(component.name.clone(), "valid".to_string(), 0)],
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

pub fn par_fsm(component: &Component) -> FSM {
    let (start, mut fsm) = FSM::new(&component.name);

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
        vec![(component.name.clone(), "valid".to_string(), 0)],
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

pub fn seq_fsm(component: &Component) -> FSM {
    let (start, mut fsm) = FSM::new(&component.name);

    let mut current = fsm.new_state();
    //transition from start to current

    fsm.get_state(start).add_transition((
        vec![(component.name.clone(), "valid".to_string(), 1)],
        current,
    ));

    let rdy_ports = port_def_to_input(
        "ready",
        component.inputs.clone(),
        component.name.clone(),
    );
    let val_ports = port_def_to_input(
        "valid",
        component.outputs.clone(),
        component.name.clone(),
    );
    assert!(rdy_ports.len() == val_ports.len());
    for i in 0..rdy_ports.len() {
        let next = fsm.new_state();
        fsm.get_state(current)
            .add_transition((vec![rdy_ports[i].clone()], next));
        fsm.get_state(current).push_output(val_ports[i].clone());
        if i == rdy_ports.len() - 1 {
            fsm.get_state(next).add_transition((
                vec![(component.name.clone(), "valid".to_string(), 0)],
                start,
            ));
            fsm.get_state(next).push_output((
                component.name.clone(),
                "ready".to_string(),
                1,
            ));
        } else {
            current = next;
        }
    }
    fsm
}

pub fn if_fsm(component: &Component) -> FSM {
    let (start, mut fsm) = FSM::new(&component.name);

    // cond state
    let cond = fsm.new_state();
    let mut cond_val_outputs = port_def_to_input(
        "cond_val",
        component.outputs.clone(),
        component.name.clone(),
    );
    fsm.get_state(cond).add_outputs(&mut cond_val_outputs);

    // transition from start to cond on valid
    fsm.get_state(start).add_transition((
        vec![(component.name.clone(), "valid".to_string(), 1)],
        cond,
    ));

    // add end state outputs and transitions
    let end = fsm.new_state();
    fsm.get_state(end).push_output((
        component.name.clone(),
        "ready".to_string(),
        1,
    ));
    fsm.get_state(end).add_transition((
        vec![(component.name.clone(), "valid".to_string(), 0)],
        start,
    ));

    //let mut current = fsm.new_state();
    let rdy_name = vec!["ready_f", "ready_t"];
    let val_name = vec!["valid_f", "valid_t"];
    for i in 0..2 {
        // 2 branches
        let rdy_port = port_def_to_input(
            rdy_name[i],
            component.inputs.clone(),
            component.name.clone(),
        );
        let val_port = port_def_to_input(
            val_name[i],
            component.outputs.clone(),
            component.name.clone(),
        );
        assert!(rdy_port.len() <= 1 && rdy_port.len() == val_port.len());
        let mut cond_rdy_ins = port_def_to_input(
            "cond_rdy",
            component.inputs.clone(),
            component.name.clone(),
        );
        cond_rdy_ins.push((
            component.name.clone(),
            "condition".to_string(),
            i as i64,
        ));
        cond_rdy_ins.push((
            component.name.clone(),
            "condition_read_in".to_string(),
            1,
        ));
        if rdy_port.len() == 1 {
            let branch = fsm.new_state();
            fsm.get_state(cond)
                .add_transition((cond_rdy_ins.clone(), branch));
            fsm.get_state(branch)
                .add_transition((vec![rdy_port[0].clone()], end));
            fsm.get_state(branch).push_output(val_port[0].clone());
        } else {
            fsm.get_state(cond)
                .add_transition((cond_rdy_ins.clone(), end));
        }
    }
    fsm
}

pub fn while_fsm(component: &Component) -> FSM {
    let (start, mut fsm) = FSM::new(&component.name);
    let cond = fsm.new_state();
    let body = fsm.new_state();
    let end = fsm.new_state();

    // transition start -> cond
    fsm.get_state(start).add_transition((
        vec![(component.name.clone(), "valid".to_string(), 1)],
        cond,
    ));
    // transition end -> start
    fsm.get_state(end).add_transition((
        vec![(component.name.clone(), "valid".to_string(), 0)],
        start,
    ));

    // set outputs for end
    fsm.get_state(end).push_output((
        component.name.clone(),
        "ready".to_string(),
        1,
    ));

    let rdy_port = port_def_to_input(
        "ready",
        component.inputs.clone(),
        component.name.clone(),
    );
    let val_port = port_def_to_input(
        "val",
        component.outputs.clone(),
        component.name.clone(),
    );

    // add cond outputs
    let mut cond_val_outputs = port_def_to_input(
        "cond_val",
        component.outputs.clone(),
        component.name.clone(),
    );
    fsm.get_state(cond).add_outputs(&mut cond_val_outputs);

    assert!(rdy_port.len() == 1 && rdy_port.len() == val_port.len());
    let mut cond_rdy_ins = port_def_to_input(
        "cond_rdy",
        component.inputs.clone(),
        component.name.clone(),
    );
    cond_rdy_ins.push((
        component.name.clone(),
        "condition_read_in".to_string(),
        1,
    ));
    let mut tbranch_conds = cond_rdy_ins.clone();
    tbranch_conds.push((component.name.clone(), "condition".to_string(), 1));
    // transition cond -> body
    fsm.get_state(cond).add_transition((tbranch_conds, body));
    // transition body -> cond
    fsm.get_state(body)
        .add_transition((vec![rdy_port[0].clone()], cond));
    fsm.get_state(body).push_output(val_port[0].clone());

    // transition condition -> end
    let mut fbranch_conds = cond_rdy_ins.clone();
    fbranch_conds.push((component.name.clone(), "condition".to_string(), 0));
    fsm.get_state(cond).add_transition((fbranch_conds, end));

    fsm
}
