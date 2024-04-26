use crate::passes::math_utilities::get_bit_width_from;
use calyx_ir::{self as ir};
use calyx_ir::{build_assignments, Nothing};
use calyx_ir::{guard, structure};
use ir::Cell;
use std::collections::HashMap;

/// Represents a static schedule.
/// A static schedule does not need transitions--it will unconditionally increment
/// by one each time.
#[derive(Debug, Default)]
pub struct StaticSchedule {
    num_states: u64,
    queries: HashMap<(u64, u64), u64>,
    pub static_groups: Vec<ir::RRC<ir::StaticGroup>>,
    pub static_group_names: Vec<ir::Id>,
}

impl StaticSchedule {
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
    pub fn gather_info(&mut self) {
        self.num_states = 0;
        for static_group in &self.static_groups {
            // Getting self.queries
            for static_assign in &static_group.borrow().assignments {
                for query in Self::queries_from_guard(&static_assign.guard) {
                    let count = self.queries.entry(query).or_insert(0);
                    *count += 1;
                }
            }
            // Getting self.num_states
            self.num_states = std::cmp::max(
                self.num_states,
                static_group.borrow().get_latency(),
            );
        }
    }

    pub fn realize_schedule(
        &mut self,
        builder: &mut ir::Builder,
    ) -> (
        Vec<Vec<ir::Assignment<Nothing>>>,
        Vec<ir::Assignment<Nothing>>,
    ) {
        let fsm_size = get_bit_width_from(
            self.num_states + 1, /* represent 0..latency */
        );
        structure!( builder;
            let fsm = prim std_reg(fsm_size);
            let signal_on = constant(1,1);
            let adder = prim std_add(fsm_size);
            let final_state = constant(self.num_states-1, fsm_size);
            let const_one = constant(1, fsm_size);
          let first_state = constant(0, fsm_size);
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
          fsm["in"] = final_state_guard ? adder["out"];
           // resets the fsm early
          fsm["in"] = not_final_state_guard ? first_state["out"];
        );
        let mut res = vec![];
        for static_group in &mut self.static_groups {
            let mut static_group_ref = static_group.borrow_mut();
            let assigns = static_group_ref
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
            res.push(assigns);
        }
        (res, fsm_incr_assigns.to_vec())
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
    // Mainly transforms the guards such that fsm.out >= 2 & fsm.out <= 3
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
