use crate::analysis::ControlId;
use calyx_ir as ir;
use std::{collections::HashMap, fmt::Debug};

#[derive(Default)]
pub struct StaticParDomination {
    pub enable_timing_map: HashMap<u64, HashMap<u64, (u64, u64)>>,
    /// name of component
    component_name: ir::Id,
}

impl Debug for StaticParDomination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "")
    }
}

impl StaticParDomination {
    /// Construct a live range analysis.
    pub fn new(control: &mut ir::Control, comp_name: ir::Id) -> Self {
        let mut timing_map = StaticParDomination {
            component_name: comp_name,
            ..Default::default()
        };
        // compute_unique_ids is deterministic
        // so if we're calling it after we've called live range analysis (and the
        // control program hasn't changed, then this is unnecessary)
        ControlId::compute_unique_ids(control, 0, false);

        timing_map.build_time_map(control);

        timing_map
    }

    // updates self.cell_map, returns the state after the invoke/enable has occured
    // assumes that there is a cur_state = (par_id, thread_id, cur_clock)
    // also, id is the id of the invoke/enable, and latency is the latency of the
    // invoke/enable
    fn update_invoke_enable(
        &mut self,
        id: u64,
        latency: u64,
        cur_state: (u64, u64),
    ) -> (u64, u64) {
        let (par_id, cur_clock) = cur_state;
        let enable_mappings = self.enable_timing_map.entry(par_id).or_default();
        // maps enable ids -> clock cycles that they're live in
        match enable_mappings.get(&id) {
            Some(_) =>
            // we already have an earlier execution of the group, so we don't care about a later execution
            {
                ()
            }
            None => {
                enable_mappings
                    .insert(id, (cur_clock, cur_clock + latency - 1));
            }
        }
        (par_id, cur_clock + latency)
    }

    // Recursively updates self.time_map
    // This is a helper function for fn `build_time_map`.
    // Read comment for that function to see what this function is doing
    fn build_time_map_static(
        &mut self,
        sc: &ir::StaticControl,
        // cur_state = Some(parent_par_id, thread_id, cur_clock) if we're inside a static par, None otherwise.
        // parent_par_id = Node ID of the static par that we're analyzing
        // cur_clock = current clock cycles we're at relative to the start of parent_par
        cur_state: Option<(u64, u64)>,
    ) -> Option<(u64, u64)> {
        match sc {
            ir::StaticControl::Empty(_) => cur_state,
            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => match cur_state {
                Some((parent_par, cur_clock)) => {
                    let latency = sc.get_latency();
                    // we already know parent par + latency of the if stmt, so don't
                    // care about return type: we just want to add enables to the timing map
                    self.build_time_map_static(tbranch, cur_state);
                    self.build_time_map_static(fbranch, cur_state);
                    Some((parent_par, cur_clock + latency))
                }
                None => {
                    // should still look thru the branches in case there are static pars
                    // inside the branches
                    self.build_time_map_static(tbranch, cur_state);
                    self.build_time_map_static(fbranch, cur_state);
                    None
                }
            },
            ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
                match cur_state {
                    Some(cur_state_unwrapped) => {
                        let enable_id = ControlId::get_guaranteed_id_static(sc);
                        let latency = group.borrow().get_latency();
                        Some(self.update_invoke_enable(
                            enable_id,
                            latency,
                            cur_state_unwrapped,
                        ))
                    }
                    None => cur_state,
                }
            }
            ir::StaticControl::Invoke(_) => {
                todo!("static invokes currently undefined")
            }
            ir::StaticControl::Repeat(ir::StaticRepeat {
                body,
                num_repeats,
                ..
            }) => {
                self.build_time_map_static(body, cur_state);
                match cur_state {
                    Some((par_id, cur_clock_cycle)) => {
                        Some((par_id, cur_clock_cycle + body.get_latency()))
                    }
                    None => None,
                }
            }
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
                // this works whether or not cur_state is None or Some
                let mut new_state = cur_state;
                for stmt in stmts {
                    new_state = self.build_time_map_static(stmt, new_state);
                }
                new_state
            }
            ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                // We know that all children must be static
                // Analyze the Current Par
                for stmt in stmts {
                    self.build_time_map_static(
                        stmt,
                        Some((ControlId::get_guaranteed_id_static(sc), 1)),
                    );
                }
                // If we have nested pars, want to get the clock cycles relative
                // to the start of both the current par and the nested par.
                // So we have the following code to possibly get the clock cycles
                // relative to the parent par.
                // Might be overkill, but trying to keep the analysis general.
                match cur_state {
                    Some((cur_parent_par, cur_clock)) => {
                        let mut max_latency = 0;
                        for stmt in stmts {
                            self.build_time_map_static(stmt, cur_state);
                            let cur_latency = stmt.get_latency();
                            max_latency =
                                std::cmp::max(max_latency, cur_latency)
                        }
                        Some((cur_parent_par, cur_clock + max_latency))
                    }
                    None => None,
                }
            }
        }
    }

    // Recursively updates self.time_map
    // Takes in Control block `c`, Live Range Analyss `live`
    // self.time_map maps par ids -> (maps of thread ids -> (maps of cells -> intervals for which
    // cells are live))
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
            ir::Control::While(ir::While { body, .. }) => {
                self.build_time_map(body);
            }
            ir::Control::Static(sc) => {
                self.build_time_map_static(sc, None);
            }
        }
    }
}
