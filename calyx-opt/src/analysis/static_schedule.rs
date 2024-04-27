use crate::passes::math_utilities::get_bit_width_from;
use calyx_ir::{self as ir};
use calyx_ir::{build_assignments, Nothing};
use calyx_ir::{guard, structure};
use std::collections::{HashMap, VecDeque};

/// Represents a static schedule.
/// A static schedule does not need transitions--it will unconditionally increment
/// by one each time.
#[derive(Debug, Default)]
pub struct StaticSchedule {
    /// Number of states for the FSM (this is just the latency)
    num_states: u64,
    /// The queries that the FSM supports, mapped to the number of times that
    /// query occurs.
    /// (e.g., lhs = %[2:3] ? rhs) means that the (2,3) entry in the map increases
    /// by one).
    queries: HashMap<(u64, u64), u64>,
    /// The static groups the FSM will schedule. It is a vec because sometimes
    /// the same FSM will handle two different static islands.
    pub static_groups: Vec<ir::RRC<ir::StaticGroup>>,
    pub static_group_names: Vec<ir::Id>,
}

impl From<Vec<ir::RRC<ir::StaticGroup>>> for StaticSchedule {
    fn from(static_groups: Vec<ir::RRC<ir::StaticGroup>>) -> Self {
        let mut schedule = Self::default();
        schedule.static_groups = static_groups;
        schedule.num_states = 0;
        for static_group in &schedule.static_groups {
            // Getting self.queries
            for static_assign in &static_group.borrow().assignments {
                for query in Self::queries_from_guard(&static_assign.guard) {
                    let count = schedule.queries.entry(query).or_insert(0);
                    *count += 1;
                }
            }
            // Getting self.num_states
            schedule.num_states = std::cmp::max(
                schedule.num_states,
                static_group.borrow().get_latency(),
            );
        }
        schedule
    }
}

impl StaticSchedule {
    /// Given a guard, returns the queries that the static "FSM" (which is
    /// really just a counter) will make.
    fn queries_from_guard(
        guard: &ir::Guard<ir::StaticTiming>,
    ) -> Vec<(u64, u64)> {
        match guard {
            ir::Guard::Or(l, r) | ir::Guard::And(l, r) => {
                let mut lvec = Self::queries_from_guard(l);
                let rvec = Self::queries_from_guard(r);
                lvec.extend(rvec);
                lvec
            }
            ir::Guard::Not(g) => Self::queries_from_guard(g),
            ir::Guard::Port(_)
            | ir::Guard::CompOp(_, _, _)
            | ir::Guard::True => vec![],
            ir::Guard::Info(static_timing) => {
                vec![static_timing.get_interval()]
            }
        }
    }

    pub fn realize_schedule(
        &mut self,
        builder: &mut ir::Builder,
    ) -> (VecDeque<Vec<ir::Assignment<Nothing>>>, (ir::Id, u64)) {
        let fsm_size = get_bit_width_from(
            self.num_states + 1, /* represent 0..latency */
        );
        // First build the fsm we will use for each static group
        let fsm = builder.add_primitive("fsm", "std_reg", &[fsm_size]);
        let fsm_name = fsm.borrow().name();
        let mut res = VecDeque::new();
        for static_group in &mut self.static_groups {
            let mut static_group_ref = static_group.borrow_mut();
            let mut assigns: Vec<ir::Assignment<Nothing>> = static_group_ref
                .assignments
                .drain(..)
                .map(|static_assign| {
                    Self::make_assign_dyn(
                        static_assign,
                        &fsm,
                        fsm_size,
                        builder,
                        false,
                        None,
                    )
                })
                .collect();

            structure!( builder;
                let signal_on = constant(1,1);
                let adder = prim std_add(fsm_size);
                let const_one = constant(1, fsm_size);
                let first_state = constant(0, fsm_size);
                let final_state = constant(static_group_ref.get_latency() - 1, fsm_size);
            );
            let not_final_state_guard: ir::Guard<ir::Nothing> =
                guard!(fsm["out"] != final_state["out"]);
            let final_state_guard: ir::Guard<ir::Nothing> =
                guard!(fsm["out"] == final_state["out"]);
            let fsm_incr_assigns = build_assignments!(
              builder;
              // increments the fsm
              adder["left"] = ? fsm["out"];
              adder["right"] = ? const_one["out"];
              fsm["write_en"] = ? signal_on["out"];
              fsm["in"] =  not_final_state_guard ? adder["out"];
               // resets the fsm early
              fsm["in"] = final_state_guard ? first_state["out"];
            );
            assigns.extend(fsm_incr_assigns);
            res.push_back(assigns);
        }
        (res, (fsm_name, fsm_size))
    }

    // Takes in a static guard `guard`, and returns equivalent dynamic guard
    // The only thing that actually changes is the Guard::Info case
    // We need to turn static_timing to dynamic guards using `fsm`.
    // E.g.: %[2:3] gets turned into fsm.out >= 2 & fsm.out < 3
    fn make_guard_dyn(
        guard: ir::Guard<ir::StaticTiming>,
        fsm: &ir::RRC<ir::Cell>,
        fsm_size: u64,
        builder: &mut ir::Builder,
        is_static_comp: bool,
        comp_sig: Option<ir::RRC<ir::Cell>>,
    ) -> Box<ir::Guard<Nothing>> {
        match guard {
            ir::Guard::Or(l, r) => Box::new(ir::Guard::Or(
                Self::make_guard_dyn(
                    *l,
                    fsm,
                    fsm_size,
                    builder,
                    is_static_comp,
                    comp_sig.clone(),
                ),
                Self::make_guard_dyn(
                    *r,
                    fsm,
                    fsm_size,
                    builder,
                    is_static_comp,
                    comp_sig,
                ),
            )),
            ir::Guard::And(l, r) => Box::new(ir::Guard::And(
                Self::make_guard_dyn(
                    *l,
                    fsm,
                    fsm_size,
                    builder,
                    is_static_comp,
                    comp_sig.clone(),
                ),
                Self::make_guard_dyn(
                    *r,
                    fsm,
                    fsm_size,
                    builder,
                    is_static_comp,
                    comp_sig,
                ),
            )),
            ir::Guard::Not(g) => {
                Box::new(ir::Guard::Not(Self::make_guard_dyn(
                    *g,
                    fsm,
                    fsm_size,
                    builder,
                    is_static_comp,
                    comp_sig,
                )))
            }
            ir::Guard::CompOp(op, l, r) => {
                Box::new(ir::Guard::CompOp(op, l, r))
            }
            ir::Guard::Port(p) => Box::new(ir::Guard::Port(p)),
            ir::Guard::True => Box::new(ir::Guard::True),
            ir::Guard::Info(static_timing) => {
                let (beg, end) = static_timing.get_interval();
                if is_static_comp && beg == 0 && end == 1 {
                    let interval_const = builder.add_constant(0, fsm_size);
                    let sig = comp_sig.unwrap();
                    let g1 = guard!(sig["go"]);
                    let g2 = guard!(fsm["out"] == interval_const["out"]);
                    let g = ir::Guard::And(Box::new(g1), Box::new(g2));
                    return Box::new(g);
                }
                if beg + 1 == end {
                    // if beg + 1 == end then we only need to check if fsm == beg
                    let interval_const = builder.add_constant(beg, fsm_size);
                    let g = guard!(fsm["out"] == interval_const["out"]);
                    Box::new(g)
                } else if beg == 0 {
                    // if beg == 0, then we only need to check if fsm < end
                    let end_const = builder.add_constant(end, fsm_size);
                    let lt: ir::Guard<Nothing> =
                        guard!(fsm["out"] < end_const["out"]);
                    Box::new(lt)
                } else {
                    // otherwise, check if fsm >= beg & fsm < end
                    let beg_const = builder.add_constant(beg, fsm_size);
                    let end_const = builder.add_constant(end, fsm_size);
                    let beg_guard: ir::Guard<Nothing> =
                        guard!(fsm["out"] >= beg_const["out"]);
                    let end_guard: ir::Guard<Nothing> =
                        guard!(fsm["out"] < end_const["out"]);
                    Box::new(ir::Guard::And(
                        Box::new(beg_guard),
                        Box::new(end_guard),
                    ))
                }
            }
        }
    }

    // Takes in static assignment `assign` and returns a dynamic assignments
    // Mainly transforms the guards from %[2:3] -> fsm.out >= 2 & fsm.out <= 3
    pub fn make_assign_dyn(
        assign: ir::Assignment<ir::StaticTiming>,
        fsm: &ir::RRC<ir::Cell>,
        fsm_size: u64,
        builder: &mut ir::Builder,
        is_static_comp: bool,
        comp_sig: Option<ir::RRC<ir::Cell>>,
    ) -> ir::Assignment<Nothing> {
        ir::Assignment {
            src: assign.src,
            dst: assign.dst,
            attributes: assign.attributes,
            guard: Self::make_guard_dyn(
                *assign.guard,
                fsm,
                fsm_size,
                builder,
                is_static_comp,
                comp_sig,
            ),
        }
    }
}
