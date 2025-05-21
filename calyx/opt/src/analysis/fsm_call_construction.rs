use calyx_ir::{self as ir, FSM, Id, PortParent, RRC};
use itertools::Itertools;
use std::{collections::HashMap, fmt};

#[derive(Debug)]

/// Alias for an FSM state
struct State(usize);

/// Alias for local FSM identifier
#[derive(Hash)]
struct NodeId(usize);

pub struct FSMCallGraph {
    /// Map from FSM to callee FSMs, divided by state
    call_graph: Vec<(Id, Vec<(State, Vec<Id>)>)>,

    /// Map from canonical representation of FSM to FSM construct
    _id2fsm: HashMap<Id, RRC<FSM>>,

    /// Map from canonical representation of FSM to index in call graph
    _id2local: HashMap<Id, NodeId>,
}

impl FSMCallGraph {
    fn _build(comp: &ir::Component) -> Self {
        let (call_graph, _id2fsm, _id2local) = comp
            .fsms
            .iter()
            .enumerate()
            .map(|(local_id, fsm)| {
                let fsm_id = fsm.borrow().name();
                let fsm_calls = fsm
                    .borrow()
                    .assignments
                    .iter()
                    .enumerate()
                    .filter_map(|(state, asgns)| {
                        let calls_from_state = asgns
                            .iter()
                            .filter_map(|asgn| {
                                // Callee FSMs are those whose [start] port is
                                // written to.
                                let port = &asgn.dst.borrow();
                                match &port.parent {
                                    PortParent::FSM(sub_fsm) => {
                                        if &port.name == "start" {
                                            Some(
                                                sub_fsm
                                                    .upgrade()
                                                    .borrow()
                                                    .name(),
                                            )
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                }
                            })
                            .collect_vec();

                        if calls_from_state.is_empty() {
                            None
                        } else {
                            Some((State(state), calls_from_state))
                        }
                    })
                    .collect();

                (
                    (fsm_id, fsm_calls),
                    (fsm_id.clone(), RRC::clone(fsm)),
                    (fsm_id, NodeId(local_id)),
                )
            })
            .multiunzip();

        Self {
            call_graph,
            _id2fsm,
            _id2local,
        }
    }
}

impl fmt::Display for FSMCallGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (fsm_id, states) in self.call_graph.iter() {
            writeln!(f, "{}", fsm_id.id)?;
            for (state, callees) in states.iter() {
                writeln!(f, "    {}: ", state.0)?;
                for callee in callees.iter() {
                    writeln!(f, "        {}", callee.id)?;
                }
            }
        }
        Ok(())
    }
}
