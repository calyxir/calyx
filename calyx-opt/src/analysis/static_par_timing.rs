use super::LiveRangeAnalysis;
use crate::analysis::ControlId;
use calyx_ir as ir;
use std::{collections::HashMap, fmt::Debug};

/// maps cell names to a vector of tuples (i,j), which is the clock
/// cycles (relative to the start of the par) that enable is live
/// the tuples/intervals should always be sorted within the vec
type CellTimingMap = HashMap<ir::Id, Vec<(u64, u64)>>;
/// maps threads (i.e., direct children of pars) to cell
/// timing maps
type ThreadTimingMap = HashMap<u64, CellTimingMap>;

#[derive(Default)]
pub struct StaticParTiming {
    /// Map from par block ids to cell_timing_maps
    cell_map: HashMap<u64, ThreadTimingMap>,
    /// name of component
    component_name: ir::Id,
}

impl Debug for StaticParTiming {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //must sort the hashmap and hashsets in order to get consistent ordering
        writeln!(
            f,
            "This maps ids of par blocks to \" cell timing maps \", which map cells to intervals (i,j), that signify the clock cycles the group is active for, \n relative to the start of the given par block"
        )?;
        write!(
            f,
            "============ Map for Component \"{}\"",
            self.component_name
        )?;
        writeln!(f, " ============")?;
        let map = self.cell_map.clone();
        // Sorting map to get deterministic ordering
        let mut vec: Vec<(u64, ThreadTimingMap)> = map.into_iter().collect();
        vec.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        for (par_id, thread_timing_map) in vec.into_iter() {
            write!(f, "========")?;
            write!(f, "Par Node ID: {:?}", par_id)?;
            writeln!(f, " ========")?;
            let mut vec1: Vec<(u64, CellTimingMap)> =
                thread_timing_map.into_iter().collect::<Vec<_>>();
            vec1.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
            for (thread_id, cell_timing_map) in vec1 {
                write!(f, "====")?;
                write!(f, "Child/Thread ID: {:?}", thread_id)?;
                writeln!(f, " ====")?;
                let mut vec2: Vec<(ir::Id, Vec<(u64, u64)>)> =
                    cell_timing_map.into_iter().collect::<Vec<_>>();
                vec2.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
                for (cell_name, clock_intervals) in vec2 {
                    write!(f, "{:?} -- ", cell_name)?;
                    writeln!(f, "{:?}", clock_intervals)?;
                }
            }
            writeln!(f)?
        }
        write!(f, "")
    }
}

impl StaticParTiming {
    /// Construct a live range analysis.
    pub fn new(
        control: &mut ir::Control,
        comp_name: ir::Id,
        live: &LiveRangeAnalysis,
    ) -> Self {
        let mut time_map = StaticParTiming {
            component_name: comp_name,
            ..Default::default()
        };

        // compute_unique_ids is deterministic
        // so if we're calling it after we've called live range analysis (and the
        // control program hasn't changed, then this is unnecessary)
        ControlId::compute_unique_ids(control, 0, false);

        time_map.build_time_map(control, None, live);

        time_map
    }

    /// `par_id` is the id of a par thread.
    /// `thread_a` and `thread_b` are ids of direct children of par_id (if `thread_a` and
    /// `thread_b` are *not* direct children of par_id, then the function will error)
    /// `a` and `b` are cell names
    /// liveness_overlaps checks if the liveness of `a` in `thread_a` ever overlaps
    /// with the liveness of `b` in `thread_b`
    /// if `par_id` is not static, then will automtically return true
    pub fn liveness_overlaps(
        &self,
        par_id: &u64,
        thread_a: &u64,
        thread_b: &u64,
        a: &ir::Id,
        b: &ir::Id,
    ) -> bool {
        // unwrapping cell_map data structure, eventually getting to the two vecs
        // a_liveness, b_liveness, that we actually care about
        let thread_timing_map = match self.cell_map.get(par_id) {
            Some(m) => m,
            // not a static par block, so must assume overlap
            None => return true,
        };
        let a_liveness = thread_timing_map
            .get(thread_a)
            .unwrap_or_else(|| {
                unreachable!("{} not a thread in {}", thread_a, par_id)
            })
            .get(a);
        let b_liveness = thread_timing_map
            .get(thread_b)
            .unwrap_or_else(|| {
                unreachable!("{} not a thread in {}", thread_a, par_id)
            })
            .get(b);
        match (a_liveness, b_liveness) {
            (Some(a_intervals), Some(b_intervals)) => {
                let mut a_iter = a_intervals.iter();
                let mut b_iter = b_intervals.iter();
                let mut cur_a = a_iter.next();
                let mut cur_b = b_iter.next();
                // this relies on the fact that a_iter and b_iter are sorted
                // in ascending order
                while cur_a.is_some() && cur_b.is_some() {
                    let ((a1, a2), (b1, b2)) = (cur_a.unwrap(), cur_b.unwrap());
                    // if a1 is smaller, checks if it overlaps with
                    // b1. If it does, return true, otherwise, advance
                    // a in the iteration
                    match a1.cmp(b1) {
                        std::cmp::Ordering::Less => {
                            if a2 >= b1 {
                                return true;
                            } else {
                                cur_a = a_iter.next();
                            }
                        }
                        std::cmp::Ordering::Greater => {
                            if b2 >= a1 {
                                return true;
                            } else {
                                cur_b = b_iter.next();
                            }
                        }
                        std::cmp::Ordering::Equal => return true,
                    }
                }
                false
            }
            _ => false,
        }
    }

    // updates self.cell_map, returns the state after the invoke/enable has occured
    // assumges that there is a cur_state = (par_id, thread_id, cur_clock)
    // also, id is the id of the invoke/enable, and latency is the latency of the
    // invoke/enable
    fn update_invoke_enable(
        &mut self,
        id: u64,
        latency: u64,
        live: &LiveRangeAnalysis,
        cur_state: (u64, u64, u64),
    ) -> (u64, u64, u64) {
        let (par_id, thread_id, cur_clock) = cur_state;
        // live set is all cells live at this invoke/enable, organized by cell type
        let live_set = live.get(&id).clone();
        // go thru all live cells in this enable add them to appropriate entry in
        // self.cell_map
        for (_, live_cells) in live_set {
            for cell in live_cells {
                let interval_vec = self
                    .cell_map
                    .entry(par_id)
                    .or_default()
                    .entry(thread_id)
                    .or_default()
                    .entry(cell)
                    .or_default();
                // we need to check whether we've already added this
                // to vec before or not. If we haven't,
                // then we can push
                // This can sometimes occur if there is a par block,
                // that contains a while loop, and that while loop
                // contains another par block.
                match interval_vec.last() {
                    None => {
                        interval_vec.push((cur_clock, cur_clock + latency - 1))
                    }
                    Some(interval) => {
                        if *interval != (cur_clock, cur_clock + latency - 1) {
                            interval_vec
                                .push((cur_clock, cur_clock + latency - 1))
                        }
                    }
                }
            }
        }
        (par_id, thread_id, cur_clock + latency)
    }

    // Recursively updates self.time_map
    // This is a helper function for fn `build_time_map`.
    // Read comment for that function to see what this function is doing
    fn build_time_map_static(
        &mut self,
        sc: &ir::StaticControl,
        // cur_state = Some(parent_par_id, thread_id, cur_clock) if we're inside a static par, None otherwise.
        // parent_par_id = Node ID of the static par that we're analyzing
        // thread_id = Node ID of the thread that we're analyzing within the par
        // note that this thread_id only corresponds to "direct" children
        // cur_clock = current clock cycles we're at relative to the start of parent_par
        cur_state: Option<(u64, u64, u64)>,
        // LiveRangeAnalysis instance
        live: &LiveRangeAnalysis,
    ) -> Option<(u64, u64, u64)> {
        match sc {
            ir::StaticControl::Empty(_) => cur_state,
            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => match cur_state {
                Some((parent_par, thread_id, cur_clock)) => {
                    let latency = sc.get_latency();
                    // we already know parent par + latency of the if stmt, so don't
                    // care about return type: we just want to add enables to the timing map
                    self.build_time_map_static(tbranch, cur_state, live);
                    self.build_time_map_static(fbranch, cur_state, live);
                    Some((parent_par, thread_id, cur_clock + latency))
                }
                None => {
                    // should still look thru the branches in case there are static pars
                    // inside the branches
                    self.build_time_map_static(tbranch, cur_state, live);
                    self.build_time_map_static(fbranch, cur_state, live);
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
                            live,
                            cur_state_unwrapped,
                        ))
                    }
                    None => cur_state,
                }
            }
            ir::StaticControl::Invoke(ir::StaticInvoke { latency, .. }) => {
                match cur_state {
                    Some(cur_state_unwrapped) => {
                        let invoke_id = ControlId::get_guaranteed_id_static(sc);
                        Some(self.update_invoke_enable(
                            invoke_id,
                            *latency,
                            live,
                            cur_state_unwrapped,
                        ))
                    }
                    None => cur_state,
                }
            }
            ir::StaticControl::Repeat(ir::StaticRepeat {
                body,
                num_repeats,
                ..
            }) => {
                if cur_state.is_some() {
                    // essentially just unrolling the loop
                    let mut new_state = cur_state;
                    for _ in 0..*num_repeats {
                        new_state =
                            self.build_time_map_static(body, new_state, live)
                    }
                    new_state
                } else {
                    // look thru while body for static pars
                    self.build_time_map_static(body, cur_state, live);
                    None
                }
            }
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
                // this works whether or not cur_state is None or Some
                let mut new_state = cur_state;
                for stmt in stmts {
                    new_state =
                        self.build_time_map_static(stmt, new_state, live);
                }
                new_state
            }
            ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                // We know that all children must be static
                // Analyze the Current Par
                for stmt in stmts {
                    self.build_time_map_static(
                        stmt,
                        Some((
                            ControlId::get_guaranteed_id_static(sc),
                            ControlId::get_guaranteed_id_static(stmt),
                            1,
                        )),
                        live,
                    );
                }
                // If we have nested pars, want to get the clock cycles relative
                // to the start of both the current par and the nested par.
                // So we have the following code to possibly get the clock cycles
                // relative to the parent par.
                // Might be overkill, but trying to keep it general.
                match cur_state {
                    Some((cur_parent_par, cur_thread, cur_clock)) => {
                        let mut max_latency = 0;
                        for stmt in stmts {
                            self.build_time_map_static(stmt, cur_state, live);
                            let cur_latency = stmt.get_latency();
                            max_latency =
                                std::cmp::max(max_latency, cur_latency)
                        }
                        Some((
                            cur_parent_par,
                            cur_thread,
                            cur_clock + max_latency,
                        ))
                    }
                    None => None,
                }
            }
        }
    }

    // Recursively updates self.time_map
    // Takes in Control block c, cur_state = Option(parent_par_id, thread_id, cur_clock)
    // parent_par_id is the id of the parent par
    // thread_id is the id of the part thread
    // cur_clock is the current clock cycle relative to the start of the parent
    // returns Option(parent_par_id, thread_id, cur_clock)
    // self.time_map currently maps par ids -> (maps of thread ids -> (maps of cells -> intervals for which
    // cells are live))
    fn build_time_map(
        &mut self,
        c: &ir::Control,
        // cur_state = Some(parent_par_id, thread_id, cur_clock) if we're inside a static par, None otherwise.
        // parent_par_id = Node ID of the static par that we're analyzing
        // thread_id = Node ID of the thread that we're analyzing within the par
        // note that this thread_id only corresponds to "direct" children
        // cur_clock = current clock cycles we're at relative to the start of parent_par
        cur_state: Option<(u64, u64, u64)>,
        // LiveRangeAnalysis instance
        live: &LiveRangeAnalysis,
    ) -> Option<(u64, u64, u64)> {
        match c {
            ir::Control::Invoke(_) => {
                if cur_state.is_some() {
                    unreachable!("no static guarantees for invoke")
                }
                cur_state
            }
            ir::Control::Empty(_) => cur_state,
            ir::Control::Enable(_) => {
                match cur_state {
                    Some(cur_state_unwrapped) => {
                        let enable_id = ControlId::get_guaranteed_id(c);
                        // add enable to self.map
                        let latency = ControlId::get_guaranteed_attribute(
                            c,
                            ir::Attribute::Static,
                        );
                        Some(self.update_invoke_enable(
                            enable_id,
                            latency,
                            live,
                            cur_state_unwrapped,
                        ))
                    }
                    None => cur_state,
                }
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                // this works whether or not cur_state is None or Some
                let mut new_state = cur_state;
                for stmt in stmts {
                    new_state = self.build_time_map(stmt, new_state, live);
                }
                new_state
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => match cur_state {
                Some((parent_par, thread_id, cur_clock)) => {
                    let tbranch_latency = ControlId::get_guaranteed_attribute(
                        tbranch,
                        ir::Attribute::Static,
                    );
                    let fbranch_latency = ControlId::get_guaranteed_attribute(
                        fbranch,
                        ir::Attribute::Static,
                    );
                    let max_latency =
                        std::cmp::max(tbranch_latency, fbranch_latency);
                    // we already know parent par + latency of the if stmt, so don't
                    // care about return type: we just want to add enables to the timing map
                    self.build_time_map(tbranch, cur_state, live);
                    self.build_time_map(fbranch, cur_state, live);
                    Some((parent_par, thread_id, cur_clock + max_latency))
                }
                None => {
                    // should still look thru the branches in case there are static pars
                    // inside the branches
                    self.build_time_map(tbranch, cur_state, live);
                    self.build_time_map(fbranch, cur_state, live);
                    None
                }
            },
            ir::Control::While(ir::While { body, .. }) => {
                if cur_state.is_some() {
                    let bound = ControlId::get_guaranteed_attribute(
                        c,
                        ir::Attribute::Bound,
                    );
                    // essentially just unrolling the loop
                    let mut new_state = cur_state;
                    for _ in 0..bound {
                        new_state = self.build_time_map(body, new_state, live)
                    }
                    new_state
                } else {
                    // look thru while body for static pars
                    self.build_time_map(body, cur_state, live);
                    None
                }
            }
            ir::Control::Par(ir::Par { stmts, attributes }) => {
                if attributes.get(ir::Attribute::Static).is_some() {
                    // Analyze the Current Par
                    for stmt in stmts {
                        self.build_time_map(
                            stmt,
                            Some((
                                ControlId::get_guaranteed_id(c),
                                ControlId::get_guaranteed_id(stmt),
                                1,
                            )),
                            live,
                        );
                    }
                    // If we have nested pars, want to get the clock cycles relative
                    // to the start of both the current par and the nested par.
                    // So we have the following code to possibly get the clock cycles
                    // relative to the parent par.
                    // Might be overkill, but trying to keep it general.
                    match cur_state {
                        Some((cur_parent_par, cur_thread, cur_clock)) => {
                            let mut max_latency = 0;
                            for stmt in stmts {
                                self.build_time_map(stmt, cur_state, live);
                                let cur_latency =
                                    ControlId::get_guaranteed_attribute(
                                        stmt,
                                        ir::Attribute::Static,
                                    );
                                max_latency =
                                    std::cmp::max(max_latency, cur_latency)
                            }
                            Some((
                                cur_parent_par,
                                cur_thread,
                                cur_clock + max_latency,
                            ))
                        }
                        None => None,
                    }
                } else {
                    // look thru par block for static pars
                    for stmt in stmts {
                        self.build_time_map(stmt, None, live);
                    }
                    None
                }
            }
            ir::Control::Static(sc) => {
                self.build_time_map_static(sc, cur_state, live)
            }
        }
    }
}
