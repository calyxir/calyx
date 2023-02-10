use crate::{
    analysis::ControlId,
    ir::{self},
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

/// maps the ids of groups to a set of tuples (i,j), the clock cycles (relative
/// to the start of the par) that group is live
type ParTimingMap = HashMap<u64, HashSet<(u64, u64)>>;

///
#[derive(Default)]
pub struct StaticParTiming {
    /// Map from from ids of par blocks to par_timing_maps
    map: HashMap<u64, ParTimingMap>,
    component_name: ir::Id,
}

impl Debug for StaticParTiming {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //must sort the hashmap and hashsets in order to get consistent ordering
        writeln!(
            f,
            "This maps ids of par blocks to \" par timing maps \", which map group ids to intervals (i,j), that signify the clock cycles the group is active for, \n relative to the start of the given par block"
        )?;
        write!(f, "======== Map for Component \"{}\"", self.component_name)?;
        writeln!(f, " ========")?;
        let map = self.map.clone();
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

        // compute_unique_ids should give same id labeling if the control is the same
        ControlId::compute_unique_ids(control, 0, false);

        time_map.build_time_map(control, None, 1);

        time_map
    }

    fn build_time_map(
        &mut self,
        c: &ir::Control,
        cur_parent_par: Option<u64>,
        cur_clock: u64,
    ) -> u64 {
        match c {
            ir::Control::Invoke(_) => {
                if cur_parent_par.is_some() {
                    unreachable!("no static guarantees for invoke")
                }
                0
            }
            ir::Control::Empty(_) => cur_clock,
            ir::Control::Enable(_) => match cur_parent_par {
                Some(par_id) => {
                    let latency =
                        ControlId::get_guaranteed_attribute(c, "static");
                    let enable_id = ControlId::get_guaranteed_id(c);
                    self.map
                        .entry(par_id)
                        .or_default()
                        .entry(enable_id)
                        .or_default()
                        .insert((cur_clock, cur_clock + latency - 1));
                    cur_clock + latency
                }
                None => 0,
            },
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                let mut new_clock = cur_clock;
                for stmt in stmts {
                    new_clock =
                        self.build_time_map(stmt, cur_parent_par, new_clock);
                }
                new_clock
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                let tbranch_new_clock =
                    self.build_time_map(tbranch, cur_parent_par, cur_clock);
                let fbranch_new_clock =
                    self.build_time_map(fbranch, cur_parent_par, cur_clock);
                std::cmp::max(tbranch_new_clock, fbranch_new_clock)
            }
            ir::Control::While(ir::While { body, .. }) => {
                let bound = ControlId::get_guaranteed_attribute(c, "bound");
                // might be inefficient bc we're basically just unrolling the
                // loop
                let mut new_clock = cur_clock;
                for _ in 0..bound {
                    new_clock =
                        self.build_time_map(body, cur_parent_par, new_clock)
                }
                new_clock
            }
            ir::Control::Par(ir::Par { stmts, attributes }) => {
                if attributes.get("static").is_some() {
                    for stmt in stmts {
                        self.build_time_map(
                            stmt,
                            Some(ControlId::get_guaranteed_id(c)),
                            1,
                        );
                    }
                    // If we have nested pars, want to get the clock cycles relative
                    // to the start of both par blocks. This is possibly overkill,
                    // but trying to keep it general.
                    if cur_parent_par.is_some() {
                        let mut max_clock = cur_clock;
                        for stmt in stmts {
                            let new_clock = self.build_time_map(
                                stmt,
                                cur_parent_par,
                                cur_clock,
                            );
                            max_clock = std::cmp::max(max_clock, new_clock);
                        }
                        max_clock
                    } else {
                        0
                    }
                } else {
                    for stmt in stmts {
                        self.build_time_map(stmt, None, 0);
                    }
                    0
                }
            }
        }
    }
}
