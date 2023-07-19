use crate::analysis::ControlId;
use calyx_ir as ir;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};
const BEGIN_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::BEGIN_ID);
#[derive(Default)]
/// Computes Dominators Across Static Pars.
/// Assumes each control stmt has already been given the appropraite IDs.
pub struct StaticParDomination {
    /// (nodes for static control can only be enables or if stmts... we don't suppport invokes yet)
    /// maps par ids -> (map of node ids -> (first interval for which node is live, relative to parent par))
    pub node_timing_map: HashMap<u64, HashMap<u64, (u64, u64)>>,
    /// maps par ids -> (map of node ids -> (first interval for which node is live, relative to parent par)), but these enables *may* execute
    pub node_maybe_timing_map: HashMap<u64, HashMap<u64, (u64, u64)>>,
    /// name of component
    component_name: ir::Id,
}

impl Debug for StaticParDomination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "This maps ids of par blocks to \"node timing maps\", which map node ids to the first interval (i,j) that the node (i.e., enable/invoke/if conditional) is active for, \n relative to the start of the given par block"
        )?;
        write!(
            f,
            "============ Map for Component \"{}\"",
            self.component_name
        )?;
        writeln!(f, " ============")?;
        let node_must_map = self.node_timing_map.clone();
        let node_may_map = self.node_maybe_timing_map.clone();
        // get all par ids and iterate thru them. Sort for deterministic ordering
        let must_par_ids: HashSet<&u64> = node_must_map.keys().collect();
        let may_par_ids: HashSet<&u64> = node_may_map.keys().collect();
        let mut par_ids: Vec<_> = must_par_ids.union(&may_par_ids).collect();
        par_ids.sort();
        for par_id in par_ids.into_iter() {
            write!(f, "========")?;
            write!(f, "Par Node ID: {:?}", par_id)?;
            writeln!(f, "========")?;
            write!(f, "====")?;
            write!(f, "MUST EXECUTE")?;
            writeln!(f, "====")?;
            // print the "must executes" for the given par id
            match node_must_map.get(par_id) {
                None => (),
                Some(map) => {
                    let mut vec1: Vec<_> = map.iter().collect();
                    // sort vec1 to get deterministic ordering
                    vec1.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
                    for (enable_id, interval) in vec1 {
                        // print enable id -- (interval)
                        write!(f, "{:?} -- ", enable_id)?;
                        writeln!(f, "{:?}", interval)?;
                    }
                }
            }
            write!(f, "====")?;
            write!(f, "MAY EXECUTE")?;
            writeln!(f, "====")?;
            // print the "may executes" for the given par id
            match node_may_map.get(par_id) {
                None => (),
                Some(map) => {
                    let mut vec1: Vec<_> = map.iter().collect();
                    // sort vec1 to get deterministic ordering
                    vec1.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
                    for (enable_id, interval) in vec1 {
                        // print enable id -- (interval)
                        write!(f, "{:?} -- ", enable_id)?;
                        writeln!(f, "{:?}", interval)?;
                    }
                }
            }
        }
        writeln!(f)
    }
}

impl StaticParDomination {
    /// Construct a live range analysis.
    pub fn new(control: &mut ir::Control, comp_name: ir::Id) -> Self {
        let mut timing_map = StaticParDomination {
            component_name: comp_name,
            ..Default::default()
        };

        timing_map.build_time_map(control);

        timing_map
    }

    /// returns a HashMap that maps node ids -> set of nodes that dominate it
    /// It will only return the node ids that are dominated within the same
    /// static par block, not all the dominators for the entire control program
    pub fn get_static_dominators(&mut self) -> HashMap<u64, HashSet<u64>> {
        let mut static_dom_map: HashMap<u64, HashSet<u64>> = HashMap::new();

        for (par_id, node_interval_mapping) in &self.node_timing_map {
            let empty_map = HashMap::new();
            // these are the nodes that *may* execute for the given par id
            let may_execute_enables =
                match self.node_maybe_timing_map.get(par_id) {
                    Some(mapping) => mapping,
                    None => &empty_map,
                };
            // Very simple/naive algorithm
            // for each "must" execute nodes, it can either dominate:
            // one of the "may" execute nodes, or dominate another "must" execute node
            // "may" execute nodes cannot dominate anybody
            for (node_id1, (_, end1)) in node_interval_mapping {
                // check if node_id1 dominates any of the "may" execute nodes
                for (may_enable, (may_beg, _)) in may_execute_enables {
                    if end1 <= may_beg {
                        static_dom_map
                            .entry(*may_enable)
                            .or_default()
                            .insert(*node_id1);
                    }
                }
                // check if node_id1 dominates any of the other "must" execute nodes
                for (enable_id2, (beg2, _)) in node_interval_mapping {
                    if end1 <= beg2 {
                        static_dom_map
                            .entry(*enable_id2)
                            .or_default()
                            .insert(*node_id1);
                    }
                }
            }
        }
        static_dom_map
    }

    // updates self.node_timing_map if guaranteed_execution is true, otherwise
    // updates self.node_maybe_timing_map.
    // Also returns the "state = (par_id, cur_clock)" after the invoke/enable has occured
    // assumes that there is a cur_state = (par_id, cur_clock)
    // also, id is the id of the node, and latency is the latency of the node
    fn update_node(
        &mut self,
        id: u64,
        latency: u64,
        cur_state: (u64, u64),
        guaranteed_execute: bool,
    ) -> (u64, u64) {
        let (par_id, cur_clock) = cur_state;
        // if we are guaranteed execution, then can update self.node_timing_map
        // otherwise we must updateself.node_maybe_timing_map
        let enable_mappings = if guaranteed_execute {
            self.node_timing_map.entry(par_id).or_default()
        } else {
            self.node_maybe_timing_map.entry(par_id).or_default()
        };
        // we already have recorded an earlier execution of the node, so we don't care about a later execution
        if enable_mappings.get(&id).is_none() {
            enable_mappings.insert(id, (cur_clock, cur_clock + latency));
        }
        (par_id, cur_clock + latency)
    }

    // Recursively updates self.enable_timing_map
    // This is a helper function for fn `build_time_map`.
    // Read comment for that function to see what this function is doing
    // returns the resulting "state"
    fn build_time_map_static(
        &mut self,
        sc: &ir::StaticControl,
        // cur_state = Some(parent_par_id, cur_clock) if we're inside a static par, None otherwise.
        // parent_par_id = Node ID of the static par that we're analyzing
        // cur_clock = current clock cycles we're at relative to the start of parent_par
        cur_state: Option<(u64, u64)>,
        // whether sc is guaranteed to execute (i.e., not in an `if` statement branch)
        guaranteed_execution: bool,
    ) -> Option<(u64, u64)> {
        match sc {
            ir::StaticControl::Empty(_) => cur_state,
            ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
                if let Some(cur_state_unwrapped) = cur_state {
                    let enable_id = ControlId::get_guaranteed_id_static(sc);
                    let latency = group.borrow().get_latency();
                    Some(self.update_node(
                        enable_id,
                        latency,
                        cur_state_unwrapped,
                        guaranteed_execution,
                    ))
                } else {
                    cur_state
                }
            }
            ir::StaticControl::Invoke(inv) => {
                if let Some(cur_state_unwrapped) = cur_state {
                    let invoke_id = ControlId::get_guaranteed_id_static(sc);
                    let latency = inv.latency;
                    Some(self.update_node(
                        invoke_id,
                        latency,
                        cur_state_unwrapped,
                        guaranteed_execution,
                    ))
                } else {
                    cur_state
                }
            }
            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => {
                // should look thru if branches to see if they have static pars
                // inside of them, but static if branches don't help us with
                // dominators across pars (since we're not sure if they execute),
                // so we can't add any enables within the if branches
                self.build_time_map_static(tbranch, cur_state, false);
                self.build_time_map_static(fbranch, cur_state, false);
                let if_id =
                    ControlId::get_guaranteed_attribute_static(sc, BEGIN_ID);
                if let Some(cur_state_unwrapped) = cur_state {
                    self.update_node(
                        if_id,
                        1,
                        cur_state_unwrapped,
                        guaranteed_execution,
                    );
                }

                cur_state.map(|(parent_par, cur_clock)| {
                    (parent_par, cur_clock + sc.get_latency())
                })
            }
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                // we only need to look thru the body once either way, since we only
                // care about the *first* execution of a node
                self.build_time_map_static(
                    body,
                    cur_state,
                    guaranteed_execution,
                );
                cur_state.map(|(par_id, cur_clock_cycle)| {
                    (par_id, cur_clock_cycle + sc.get_latency())
                })
            }
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
                // this works whether or not cur_state is None or Some
                let mut new_state = cur_state;
                for stmt in stmts {
                    new_state = self.build_time_map_static(
                        stmt,
                        new_state,
                        guaranteed_execution,
                    );
                }
                new_state
            }
            ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                // We know that all children must be static
                // Analyze the Current Par
                for stmt in stmts {
                    self.build_time_map_static(
                        stmt,
                        Some((ControlId::get_guaranteed_id_static(sc), 0)),
                        true,
                    );
                }
                // If we have nested pars, want to get the clock cycles relative
                // to the start of both the current par and the nested par.
                // So we have the following code to possibly get the clock cycles
                // relative to the parent par.
                match cur_state {
                    Some((cur_parent_par, cur_clock)) => {
                        for stmt in stmts {
                            self.build_time_map_static(
                                stmt,
                                cur_state,
                                guaranteed_execution,
                            );
                        }
                        Some((cur_parent_par, cur_clock + sc.get_latency()))
                    }
                    None => None,
                }
            }
        }
    }

    // Recursively updates self.node_timing_map and self.node_maybe_timing_map
    // Takes in Control block `c`
    // they both map maps par ids -> (maps of node ids -> (first interval for which the node is live))
    fn build_time_map(&mut self, c: &ir::Control) {
        match c {
            ir::Control::Invoke(_)
            | ir::Control::Empty(_)
            | ir::Control::Enable(_) => (),
            ir::Control::Par(ir::Par { stmts, .. })
            | ir::Control::Seq(ir::Seq { stmts, .. }) => {
                for stmt in stmts {
                    self.build_time_map(stmt)
                }
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                self.build_time_map(tbranch);
                self.build_time_map(fbranch);
            }
            ir::Control::While(ir::While { body, .. })
            | ir::Control::Repeat(ir::Repeat { body, .. }) => {
                self.build_time_map(body);
            }
            ir::Control::Static(sc) => {
                self.build_time_map_static(sc, None, true);
            }
        }
    }
}
