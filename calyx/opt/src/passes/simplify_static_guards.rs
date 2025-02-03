use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;

#[derive(Default)]
/// Simplifies Static Guards
/// In particular if g = g1 & g2 & ...gn, then it takes all of the g_i's that
/// are "static timing intervals", e.g., %[2:3], and combines them into one
/// timing interval.
/// For example: (port.out | !port1.out) & (port2.out == port3.out) & %[2:8] & %[5:10] ?
/// becomes (port.out | !port1.out) & (port2.out == port3.out) & %[5:8] ?
/// by "combining" %[2:8] & %[5:10]
pub struct SimplifyStaticGuards;

impl Named for SimplifyStaticGuards {
    fn name() -> &'static str {
        "simplify-static-guards"
    }

    fn description() -> &'static str {
        "Simplify Static Guards"
    }
}

impl SimplifyStaticGuards {
    /// takes in g, and separates the "anded intervals" from the rest of the guard.
    /// In other words, if we can rewrite g as g1 & g2 & .... gn, then
    /// we take all of the g_i's that are static timing intervals (e.g., %[2:3])
    /// and return a vec of (u64, u64)s. We also return the Some(rest of guard) (i.e.,
    /// the parts that aren't "anded" intervals) if they exist
    /// e.g.:
    /// port.out & port1.out & %[3:5] & %[4:6] -> Some(port.out & port1.out), vec[(3,5), (4,6)]
    /// %[3:5] & %[4:6] -> None, vec[(3,5), (4,6)]
    pub fn separate_anded_intervals(
        g: ir::Guard<ir::StaticTiming>,
        cur_anded_intervals: &mut Vec<(u64, u64)>,
    ) -> Option<ir::Guard<ir::StaticTiming>> {
        match g {
            ir::Guard::And(g1, g2) => {
                // recursively call separate_anded_intervals on g1 and g2
                let rest_g1 =
                    Self::separate_anded_intervals(*g1, cur_anded_intervals);
                let rest_g2 =
                    Self::separate_anded_intervals(*g2, cur_anded_intervals);
                match (rest_g1, rest_g2) {
                    // both g1 and g2 are entirely made up of static timing guards
                    (None, None) => None,
                    // one of g1 or g2 is made up entirely of static timing guards
                    (None, Some(g)) | (Some(g), None) => Some(g),
                    // both g1 and g2 have non-static timing guards
                    (Some(g1_unwrapped), Some(g2_unwrapped)) => {
                        Some(ir::Guard::And(
                            Box::new(g1_unwrapped),
                            Box::new(g2_unwrapped),
                        ))
                    }
                }
            }
            ir::Guard::Info(static_timing_info) => {
                // no "rest of guard" for static intervals
                cur_anded_intervals.push(static_timing_info.get_interval());
                None
            }
            ir::Guard::True
            | ir::Guard::CompOp(_, _, _)
            | ir::Guard::Not(_)
            | ir::Guard::Or(_, _)
            | ir::Guard::Port(_) => Some(g),
        }
    }

    /// Takes in a guard and returns the simplified guard
    /// In particular if g = g1 & g2 & ...gn, then it takes all of the g_i's that
    /// are "static timing intervals", e.g., %[2:3], and combines them into one
    /// timing interval.
    /// For example: (port.out | !port1.out) & (port2.out == port3.out) & %[2:8] & %[5:10] ?
    /// becomes (port.out | !port1.out) & (port2.out == port3.out) & %[5:8] ?
    /// by "combining: %[2:8] & %[5:10]
    fn simplify_anded_guards(
        guard: ir::Guard<ir::StaticTiming>,
        group_latency: u64,
    ) -> ir::Guard<ir::StaticTiming> {
        // get the rest of the guard and the "anded intervals"
        let mut anded_intervals = Vec::new();
        let rest_guard =
            Self::separate_anded_intervals(guard, &mut anded_intervals);
        // first simplify the vec of `anded_intervals` into a single interval
        let replacing_interval = {
            if anded_intervals.is_empty() {
                // if there were no static timing guards (i.e., no %[2:3]), then
                // there is no "replacing intervals"
                None
            } else {
                // the replacing intervals should just be the latest beginning interval
                // combined with the earliest ending interval, since we know that all of
                // the intervals are connected by &.
                let (mut max_beg, mut min_end) = anded_intervals.pop().unwrap();
                for (cur_beg, cur_end) in anded_intervals {
                    max_beg = std::cmp::max(cur_beg, max_beg);
                    min_end = std::cmp::min(cur_end, min_end);
                }
                if max_beg >= min_end {
                    // if the vec was something like %[2:3] & %[4:5], then this is always false
                    // if max_beg >= min_end, then guard is always false
                    return ir::Guard::Not(Box::new(ir::Guard::True));
                } else if max_beg == 0 && min_end == group_latency {
                    // if guard will just be [0:group_latency] then it's not necessary
                    None
                } else {
                    // otherwise return the single interval as the "new" interval
                    Some(ir::Guard::Info(ir::StaticTiming::new((
                        max_beg, min_end,
                    ))))
                }
            }
        };

        // now based on `rest_guard` and `replacing_interval` we create the final guard
        match (rest_guard, replacing_interval) {
            (None, None) => ir::Guard::True,
            (None, Some(g)) | (Some(g), None) => g,
            (Some(rg), Some(ig)) => ir::Guard::And(Box::new(rg), Box::new(ig)),
        }
    }

    fn simplify_guard(
        guard: ir::Guard<ir::StaticTiming>,
        group_latency: u64,
    ) -> ir::Guard<ir::StaticTiming> {
        match guard {
            ir::Guard::Not(g) => ir::Guard::Not(Box::new(
                Self::simplify_guard(*g, group_latency),
            )),
            ir::Guard::Or(g1, g2) => ir::Guard::Or(
                Box::new(Self::simplify_guard(*g1, group_latency)),
                Box::new(Self::simplify_guard(*g2, group_latency)),
            ),
            ir::Guard::And(_, _) => {
                Self::simplify_anded_guards(guard, group_latency)
            }
            ir::Guard::Info(_) => {
                Self::simplify_anded_guards(guard, group_latency)
            }
            ir::Guard::Port(_)
            | ir::Guard::True
            | ir::Guard::CompOp(_, _, _) => guard,
        }
    }
}

impl Visitor for SimplifyStaticGuards {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        for group in comp.get_static_groups().iter() {
            let group_latency = group.borrow().get_latency();
            group
                .borrow_mut()
                .assignments
                .iter_mut()
                .for_each(|assign| {
                    assign.guard.update(|guard| {
                        Self::simplify_guard(guard, group_latency)
                    })
                });
        }

        // we don't need to traverse control
        Ok(Action::Stop)
    }
}
