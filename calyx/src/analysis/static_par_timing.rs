use crate::{
    analysis::ControlId,
    ir::{self},
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

/// maps the node ids of enables to a set of tuples (i,j), which is clock cycles (relative
/// to the start of the par) that enable is live
type ParTimingMap = HashMap<u64, HashSet<(u64, u64)>>;

#[derive(Default)]
pub struct StaticParTiming {
    /// Map from from par block ids to par_timing_maps
    map: HashMap<u64, ParTimingMap>,
    /// name of component
    component_name: ir::Id,
}

impl Debug for StaticParTiming {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //must sort the hashmap and hashsets in order to get consistent ordering
        writeln!(
            f,
            "This maps ids of par blocks to \" par timing maps \", which map enable ids to intervals (i,j), that signify the clock cycles the group is active for, \n relative to the start of the given par block"
        )?;
        write!(f, "======== Map for Component \"{}\"", self.component_name)?;
        writeln!(f, " ========")?;
        let map = self.map.clone();
        // Sorting map to get deterministic ordering
        let mut vec: Vec<(u64, ParTimingMap)> = map.into_iter().collect();
        vec.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        for (par_id, par_timing_map) in vec.into_iter() {
            write!(f, "====")?;
            write!(f, "Par Node ID: {:?}", par_id)?;
            writeln!(f, "====")?;
            let mut vec1: Vec<(u64, HashSet<(u64, u64)>)> =
                par_timing_map.into_iter().collect::<Vec<_>>();
            vec1.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
            for (group_id, clock_intervals) in vec1 {
                write!(f, "Group Node ID: {:?} --", group_id)?;
                let mut vec2: Vec<(u64, u64)> =
                    clock_intervals.into_iter().collect();
                vec2.sort_unstable();
                writeln!(f, " {:?}", vec2)?;
            }
        }
        write!(f, "}}")
    }
}

impl StaticParTiming {
    /// Construct a live range analysis.
    pub fn new(control: &mut ir::Control, comp_name: ir::Id) -> Self {
        let mut time_map = StaticParTiming {
            component_name: comp_name,
            ..Default::default()
        };

        // compute_unique_ids is deterministic
        // so if we're calling it after we've called live range analysis (and the
        // control program hasn't changed, then this is unnecessary)
        ControlId::compute_unique_ids(control, 0, false);

        time_map.build_time_map(control, None);

        time_map
    }

    // Recursively updates self.time_map
    fn build_time_map(
        &mut self,
        c: &ir::Control,
        // cur_state = Some(parent_par_id, cur_clock) if we're inside a static par, None otherwise.
        // parent_par_id = Node ID of the static par that we're analyzing
        // cur_clock = current clock cycles we're at relative to the start of parent_par
        cur_state: Option<(u64, u64)>,
    ) -> Option<(u64, u64)> {
        match c {
            ir::Control::Invoke(_) => {
                if cur_state.is_some() {
                    unreachable!("no static guarantees for invoke")
                }
                cur_state
            }
            ir::Control::Empty(_) => cur_state,
            ir::Control::Enable(_) => match cur_state {
                Some((par_id, cur_clock)) => {
                    // add enable to self.map
                    let latency =
                        ControlId::get_guaranteed_attribute(c, "static");
                    let enable_id = ControlId::get_guaranteed_id(c);
                    self.map
                        .entry(par_id)
                        .or_default()
                        .entry(enable_id)
                        .or_default()
                        .insert((cur_clock, cur_clock + latency - 1));
                    Some((par_id, cur_clock + latency))
                }
                None => cur_state,
            },
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                // this works whether or not cur_state is None or Some
                let mut new_state = cur_state;
                for stmt in stmts {
                    new_state = self.build_time_map(stmt, new_state);
                }
                new_state
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => match cur_state {
                Some((parent_par, cur_clock)) => {
                    let tbranch_latency =
                        ControlId::get_guaranteed_attribute(tbranch, "static");
                    let fbranch_latency =
                        ControlId::get_guaranteed_attribute(fbranch, "static");
                    let max_latency =
                        std::cmp::max(tbranch_latency, fbranch_latency);
                    // we already know parent par + latency of the if stmt, so don't
                    // care about return type: we just want to add enables to the timing map
                    self.build_time_map(tbranch, cur_state);
                    self.build_time_map(fbranch, cur_state);
                    Some((parent_par, cur_clock + max_latency))
                }
                None => {
                    // should still look thru the branches in case there are static pars
                    // inside the branches
                    self.build_time_map(tbranch, cur_state);
                    self.build_time_map(fbranch, cur_state);
                    None
                }
            },
            ir::Control::While(ir::While { body, .. }) => {
                if cur_state.is_some() {
                    let bound = ControlId::get_guaranteed_attribute(c, "bound");
                    // essentially just unrolling the loop
                    let mut new_state = cur_state;
                    for _ in 0..bound {
                        new_state = self.build_time_map(body, new_state)
                    }
                    new_state
                } else {
                    // look thru while body for static pars
                    self.build_time_map(body, cur_state);
                    None
                }
            }
            ir::Control::Par(ir::Par { stmts, attributes }) => {
                if attributes.get("static").is_some() {
                    // Analyze the Current Par
                    for stmt in stmts {
                        self.build_time_map(
                            stmt,
                            Some((ControlId::get_guaranteed_id(c), 1)),
                        );
                    }
                    // If we have nested pars, want to get the clock cycles relative
                    // to the start of both the current par and the nested par.
                    // So we have the following code to possibly get the clock cycles
                    // relative to the parent par.
                    // Might be overkill, but trying to keep it general.
                    match cur_state {
                        Some((cur_parent_par, cur_clock)) => {
                            let mut max_latency = 0;
                            for stmt in stmts {
                                self.build_time_map(stmt, cur_state);
                                let cur_latency =
                                    ControlId::get_guaranteed_attribute(
                                        stmt, "static",
                                    );
                                max_latency =
                                    std::cmp::max(max_latency, cur_latency)
                            }
                            Some((cur_parent_par, cur_clock + max_latency))
                        }
                        None => None,
                    }
                } else {
                    // look thru par block for static pars
                    for stmt in stmts {
                        self.build_time_map(stmt, None);
                    }
                    None
                }
            }
        }
    }
}
