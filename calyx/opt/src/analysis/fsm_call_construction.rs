use calyx_ir::{self as ir, FSM, Id, PortParent, RRC};
use itertools::Itertools;
use std::{collections::HashMap, fmt};

type Callees = Vec<(State, Vec<Id>)>;
/// Alias for an FSM state
#[derive(Clone)]
struct State(usize);

/// Alias for local FSM identifier
#[derive(Hash)]
struct NodeId(usize);

pub struct FSMCallGraph {
    /// Map from FSM to callee FSMs, divided by state
    call_graph: Vec<(Id, Callees)>,

    /// Map from canonical representation of FSM to FSM construct
    id2fsm: HashMap<Id, RRC<FSM>>,

    /// Map from canonical representation of FSM to index in call graph
    _id2local: HashMap<Id, NodeId>,
}

impl FSMCallGraph {
    fn _build(comp: &ir::Component) -> Self {
        let (call_graph, id2fsm, _id2local) = comp
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
                                match (&port.parent, &port.name == "start") {
                                    (PortParent::FSM(sub_fsm), true) => {
                                        Some(sub_fsm.upgrade().borrow().name())
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
                    (fsm_id, RRC::clone(fsm)),
                    (fsm_id, NodeId(local_id)),
                )
            })
            .multiunzip();

        Self {
            call_graph,
            id2fsm,
            _id2local,
        }
    }

    fn compact_parallel_fsms(&self, mut fsms: Vec<Id>) -> Option<Id> {
        let (mut encompassing_fsm_id_opt, mut non_encompassing_fsms) =
            (None, Vec::new());

        for lockstep_fsm in fsms.drain(..) {
            match encompassing_fsm_id_opt {
                None => {
                    encompassing_fsm_id_opt = Some(lockstep_fsm);
                }
                Some(encompassing_fsm) => {
                    if self
                        .id2fsm
                        .get(&encompassing_fsm)
                        .unwrap()
                        .borrow()
                        .size()
                        < self
                            .id2fsm
                            .get(&lockstep_fsm)
                            .unwrap()
                            .borrow()
                            .size()
                    {
                        encompassing_fsm_id_opt = Some(lockstep_fsm);
                        non_encompassing_fsms.push(encompassing_fsm);
                    } else {
                        non_encompassing_fsms.push(lockstep_fsm);
                    }
                }
            }
        }

        match encompassing_fsm_id_opt {
            None => None,
            Some(encompassing_fsm_id) => {
                let mut encompassing_fsm =
                    self.id2fsm.get(&encompassing_fsm_id).unwrap().borrow_mut();

                for parallel_fsm_id in non_encompassing_fsms.into_iter() {
                    self.id2fsm
                        .get(&parallel_fsm_id)
                        .unwrap()
                        .borrow_mut()
                        .assignments
                        .drain(..)
                        .enumerate()
                        .for_each(|(state, assigns_at_state)| {
                            encompassing_fsm
                                .assignments
                                .get_mut(state)
                                .unwrap()
                                .extend(assigns_at_state);
                        });
                }
                Some(encompassing_fsm_id)
            }
        }
    }

    fn compact_parallel_calls(&mut self) {
        let compacted_call_graph = self
            .call_graph
            .iter()
            .map(|(fsm, states)| {
                let compacted_callees = states
                    .into_iter()
                    .map(|(state, callees)| {
                        let (lockstep_fsms, mut self_looping_fsms) =
                            callees.into_iter().partition(|callee| {
                                self.id2fsm
                                    .get(callee)
                                    .unwrap()
                                    .borrow()
                                    .transitions
                                    .iter()
                                    .all(|trans| {
                                        matches!(
                                            trans,
                                            ir::Transition::Unconditional(_)
                                        )
                                    })
                            });

                        match self.compact_parallel_fsms(lockstep_fsms) {
                            None => (state.clone(), self_looping_fsms),
                            Some(compacted_fsm) => {
                                self_looping_fsms.push(compacted_fsm);
                                (state.clone(), self_looping_fsms)
                            }
                        }
                    })
                    .collect();
                (fsm.clone(), compacted_callees)
            })
            .collect();

        self.call_graph = compacted_call_graph;
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
