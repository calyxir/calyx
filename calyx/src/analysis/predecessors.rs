use std::{collections::HashMap, rc::Rc};

use crate::ir::{self, RRC};

type PredMap = HashMap<u64, ir::Guard>;

/// Calculate the ancestors for each control node and the guard that needs to be true for each
/// ancestor to transition.
///
/// For the following program:
/// ```text
/// @id(0) cond0
/// if lt.out { @id(1) true } else { @id(2) false }
/// @id(3) upd1
/// ```
/// The predecessor map will contain:
/// ```text
/// 0 -> []
/// 1 -> [(0, lt.out)]
/// 2 -> [(0, !lt.out)]
/// 3 -> [(2, 1'd1), (3, 1'd1)]
/// ```
///
/// To construct [Predecessor], ensure that [ir::Component::node_ids_valid] is
/// true by first running [ir::Component::control_with_node_ids].
#[derive(Default)]
pub struct Predecessor {
    /// Mapping from node_id to the related [ir::Group]
    group_map: HashMap<u64, RRC<ir::Group>>,
    /// Mapping from node_id of the control node to mapping from ancestor states to the Guard that
    /// need to be true for the transition to occur.
    map: HashMap<u64, PredMap>,
}

impl Predecessor {
    /// Get the predecessors for node with ID `node_id`.
    pub fn get(&self, node_id: &u64) -> Option<&PredMap> {
        self.map.get(node_id)
    }

    /// Returns the full guard that needs to be true for the transition to
    /// occur.
    /// Returns None if any of the predecessors is an [ir::Invoke] because it is
    /// not possible to structurally transition from it.
    pub fn get_guarded(&self, node_id: &u64) -> Option<PredMap> {
        self.map.get(node_id).and_then(|preds| {
            preds
                .clone()
                .into_iter()
                .map(|(st, g)| {
                    self.group_map.get(&st).map(|group| {
                        let done_cond: ir::Guard =
                            group.borrow().get("done").into();
                        (st, g & done_cond)
                    })
                })
                .collect::<Option<_>>()
        })
    }

    /// Return the predecessor map for the control program.
    pub fn pred_map(self) -> HashMap<u64, PredMap> {
        self.map
    }
}

/// Computes the exit points of a given [ir::Control] program.
///
/// ## Example
/// In the following Calyx program:
/// ```
/// while comb_reg.out {
///   seq {
///     incr;
///     cond0;
///   }
/// }
/// ```
/// The exit point is `cond0`.
///
/// Multiple exit points are created when conditions are used:
/// ```
/// while comb_reg.out {
///   incr;
///   if comb_reg2.out {
///     true;
///   } else {
///     false;
///   }
/// }
/// ```
/// The exit set is `[true, false]`.
fn control_exits(con: &ir::Control, exits: &mut Vec<u64>) {
    match con {
        ir::Control::Invoke(ir::Invoke { attributes, .. })
        | ir::Control::Enable(ir::Enable { attributes, .. }) => {
            let cur_state = attributes
                .get("node_id")
                .expect("Group does not have node_id");
            exits.push(*cur_state)
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            if let Some(con) = stmts.iter().last() {
                control_exits(con, exits);
            }
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            control_exits(tbranch, exits);
            control_exits(fbranch, exits)
        }
        ir::Control::While(ir::While { body, .. }) => {
            control_exits(body, exits)
        }
        ir::Control::Empty(_) => (),
        // Par nodes cannot enter or exit directly. They transition using
        // a special parallel control flow block.
        ir::Control::Par(_) => (),
    }
}

fn construct_map(
    con: &ir::Control,
    prev_states: PredMap,
    preds: &mut Predecessor,
) -> PredMap {
    match con {
        ir::Control::Par(_) | ir::Control::Empty(_) => prev_states,
        ir::Control::Enable(ir::Enable {
            group, attributes, ..
        }) => {
            let node_id = attributes
                .get("node_id")
                .expect("Group does not have `node_id` attribute");
            preds.map.insert(*node_id, prev_states);
            preds.group_map.insert(*node_id, Rc::clone(group));
            HashMap::new()
        }
        ir::Control::Invoke(ir::Invoke { attributes, .. }) => {
            let node_id = attributes
                .get("node_id")
                .expect("Group does not have `node_id` attribute");
            preds.map.insert(*node_id, prev_states);
            HashMap::new()
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            let mut prev = prev_states;
            for stmt in stmts {
                prev = construct_map(stmt, prev, preds);
            }
            prev
        }
        ir::Control::If(ir::If {
            tbranch,
            fbranch,
            port,
            ..
        }) => {
            let port_guard: ir::Guard = Rc::clone(port).into();
            let tru_prev = prev_states
                .clone()
                .into_iter()
                .map(|(st, g)| (st, g & port_guard.clone()))
                .collect();
            let tru_nxt = construct_map(tbranch, tru_prev, preds);

            let fal_prev = prev_states
                .into_iter()
                .map(|(st, g)| (st, g & !port_guard.clone()))
                .collect();
            let fal_nxt = construct_map(fbranch, fal_prev, preds);
            tru_nxt.into_iter().chain(fal_nxt.into_iter()).collect()
        }
        ir::Control::While(ir::While { body, port, .. }) => {
            let port_guard: ir::Guard = Rc::clone(port).into();
            // Compute the exit nodes of the body
            let mut exits = vec![];
            control_exits(body, &mut exits);

            // The predecessors for the body include the exits for the while
            // loop when the loop guard is true.
            let back_prevs = exits.into_iter().map(|st| (st, ir::Guard::True));
            let all_prevs: HashMap<_, _> =
                prev_states.into_iter().chain(back_prevs).collect();
            let body_prevs: HashMap<_, _> = all_prevs
                .clone()
                .into_iter()
                .map(|(st, g)| (st, g & port_guard.clone()))
                .collect();

            // Construct the map but ignore the returned prev_map because `all_prevs`
            // already contains body exits.
            construct_map(body, body_prevs, preds);

            all_prevs
                .into_iter()
                .map(|(st, g)| (st, g & !port_guard.clone()))
                .collect()
        }
    }
}

impl From<&ir::Control> for Predecessor {
    fn from(con: &ir::Control) -> Self {
        let mut preds = Self::default();
        construct_map(con, HashMap::new(), &mut preds);
        preds
    }
}
