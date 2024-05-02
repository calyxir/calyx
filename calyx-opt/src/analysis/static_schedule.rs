use crate::passes::math_utilities::get_bit_width_from;
use calyx_ir::{self as ir};
use calyx_ir::{build_assignments, Nothing};
use calyx_ir::{guard, structure};
use ir::Guard;
use itertools::Itertools;
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
// Define an enum called Fruit
enum FSMEncoding {
    Binary,
    _OneHot,
}

#[derive(Debug)]
pub struct StaticFSM {
    pub _num_states: u64,
    _encoding: FSMEncoding,
    // The fsm's bitwidth (this redundant information bc  we have `cell`)
    // but makes it easier if we easily have access to this.
    bitwidth: u64,
    // The actual register
    cell: ir::RRC<ir::Cell>,
}
impl StaticFSM {
    // Builds a static_fsm from: num_states and encoding type.
    fn from_basic_info(
        num_states: u64,
        encoding: FSMEncoding,
        builder: &mut ir::Builder,
    ) -> Self {
        // Only support Binary encoding currently.
        assert!(matches!(encoding, FSMEncoding::Binary));
        // First build the fsm we will use to realize the schedule.
        let fsm_size =
            get_bit_width_from(num_states + 1 /* represent 0..latency */);
        let fsm = builder.add_primitive("fsm", "std_reg", &[fsm_size]);
        StaticFSM {
            _num_states: num_states,
            _encoding: encoding,
            bitwidth: fsm_size,
            cell: fsm,
        }
    }

    // Returns assignments that make the current fsm count to n
    // and then reset back to 0.
    // `incr_condition`` is an optional guard: if it is none, then the fsm will
    // unconditionally increment.
    // If it actually holds a `guard`, then we will only start counting once
    // the condition holds.
    // (NOTE: if the guard is true while we are counting up we will just
    // ignore that guard and keep on counting-- we don't reset or anything.
    // The guard is just there to make sure we only go from 0->1 when appropriate.)
    pub fn count_to_n(
        &self,
        builder: &mut ir::Builder,
        n: u64,
        incr_condition: Option<Guard<Nothing>>,
    ) -> Vec<ir::Assignment<Nothing>> {
        // Only support Binary encoding currently.
        assert!(matches!(self._encoding, FSMEncoding::Binary));
        // Add assignments to increment the fsm by one unconditionally.
        structure!( builder;
            // done hole will be undefined bc of early reset
            let signal_on = constant(1,1);
            let adder = prim std_add(self.bitwidth);
            let const_one = constant(1, self.bitwidth);
            let first_state = constant(0, self.bitwidth);
            let final_state = constant(n, self.bitwidth);
        );
        let fsm_cell = Rc::clone(&self.cell);
        let final_state_guard: ir::Guard<Nothing> =
            guard!(fsm_cell["out"] == final_state["out"]);
        match incr_condition {
            None => {
                // "Normal" logic to increment FSM by one.
                let not_final_state_guard: ir::Guard<ir::Nothing> =
                    guard!(fsm_cell["out"] != final_state["out"]);
                build_assignments!(
                  builder;
                  // increments the fsm
                  adder["left"] = ? fsm_cell["out"];
                  adder["right"] = ? const_one["out"];
                  fsm_cell["write_en"] = ? signal_on["out"];
                  fsm_cell["in"] =  not_final_state_guard ? adder["out"];
                   // resets the fsm early
                   fsm_cell["in"] = final_state_guard ? first_state["out"];
                )
                .to_vec()
            }
            Some(condition_guard) => {
                let first_state_guard: ir::Guard<Nothing> =
                    guard!(fsm_cell["out"] == first_state["out"]);
                let cond_and_first_state =
                    ir::Guard::and(condition_guard, first_state_guard);
                let not_first_state: ir::Guard<Nothing> =
                    guard!(fsm_cell["out"] != first_state["out"]);
                let not_last_state: ir::Guard<Nothing> =
                    guard!(fsm_cell["out"] != final_state["out"]);
                let in_between_guard =
                    ir::Guard::and(not_first_state, not_last_state);
                build_assignments!(
                  builder;
                  // Incrementsthe fsm
                  adder["left"] = ? fsm_cell["out"];
                  adder["right"] = ? const_one["out"];
                  // Always write into fsm.
                  fsm_cell["write_en"] = ? signal_on["out"];
                  // If fsm == 0 and cond is high, then we start an execution.
                  fsm_cell["in"] = cond_and_first_state ? const_one["out"];
                  // If 1 < fsm < n - 1, then we unconditionally increment the fsm.
                  fsm_cell["in"] = in_between_guard ? adder["out"];
                  // If fsm == n -1 , then we reset the FSM.
                  fsm_cell["in"] = final_state_guard ? first_state["out"];
                  // Otherwise the FSM is not assigned to, so it defaults to 0.
                  // If we want, we could add an explicit assignment here that sets it
                  // to zero.
                )
                .to_vec()
            }
        }
    }

    // Returns a guard that takes a (beg, end) `query`, and returns the equivalent
    // guard to `beg <= fsm.out < end`.
    pub fn query_between(
        &self,
        builder: &mut ir::Builder,
        query: (u64, u64),
    ) -> Box<ir::Guard<Nothing>> {
        // Only support Binary encoding currently.
        assert!(matches!(self._encoding, FSMEncoding::Binary));
        let (beg, end) = query;
        let fsm_cell = Rc::clone(&self.cell);
        if beg + 1 == end {
            // if beg + 1 == end then we only need to check if fsm == beg
            let interval_const = builder.add_constant(beg, self.bitwidth);
            let g = guard!(fsm_cell["out"] == interval_const["out"]);
            Box::new(g)
        } else if beg == 0 {
            // if beg == 0, then we only need to check if fsm < end
            let end_const = builder.add_constant(end, self.bitwidth);
            let lt: ir::Guard<Nothing> =
                guard!(fsm_cell["out"] < end_const["out"]);
            Box::new(lt)
        } else {
            // otherwise, check if fsm >= beg & fsm < end
            let beg_const = builder.add_constant(beg, self.bitwidth);
            let end_const = builder.add_constant(end, self.bitwidth);
            let beg_guard: ir::Guard<Nothing> =
                guard!(fsm_cell["out"] >= beg_const["out"]);
            let end_guard: ir::Guard<Nothing> =
                guard!(fsm_cell["out"] < end_const["out"]);
            Box::new(ir::Guard::And(Box::new(beg_guard), Box::new(end_guard)))
        }
    }

    // Return an `ir::RRC<ir::Cell>`` of the fsm object.
    pub fn get_name(&self) -> ir::Id {
        self.cell.borrow().name()
    }
    // Return the bitwidth of an FSM object
    pub fn get_bitwidth(&self) -> u64 {
        self.bitwidth
    }
}

/// Represents a static schedule.
/// A static schedule does not need transitions--it will unconditionally increment
/// by one each time.
#[derive(Debug, Default)]
pub struct StaticSchedule {
    /// Number of states for the FSM
    /// (this is just the latency of the static island-- or that of the largest
    /// static island, if there are multiple islands)
    num_states: u64,
    /// The queries that the FSM needs to support.
    /// E.g., `lhs = %[2:3] ? rhs` corresponds to (2,3).
    queries: HashSet<(u64, u64)>,
    /// The static groups the FSM will schedule. It is a vec because sometimes
    /// the same FSM will handle two different static islands.
    pub static_groups: Vec<ir::RRC<ir::StaticGroup>>,
}

impl From<Vec<ir::RRC<ir::StaticGroup>>> for StaticSchedule {
    /// Builds a StaticSchedule object from a vec of static groups.
    fn from(static_groups: Vec<ir::RRC<ir::StaticGroup>>) -> Self {
        let mut schedule = StaticSchedule {
            static_groups,
            ..Default::default()
        };
        schedule.num_states = 0;
        for static_group in &schedule.static_groups {
            // Getting self.queries
            for static_assign in &static_group.borrow().assignments {
                for query in Self::queries_from_guard(&static_assign.guard) {
                    schedule.queries.insert(query);
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
    /// Given a guard, returns the queries that the static FSM (i.e., counter)
    /// will make.
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

    /// Realizes a StaticSchedule (i.e., instantiates the FSMs)
    /// If self.static_groups = vec![group1, group2, group3, ...]
    /// Then `realize_schedule()` returns vecdeque![a1, a2, a3]
    /// Where a1 are the assignments for group1, a2 are the assignments
    /// to group2, etc.
    /// It also returns the FSM in the for of an RRC<Cell>.
    ///
    /// We also have a bool argument `static_component_interface`.
    /// If you are the entire control of a static component, it is slightly different,
    /// because we need to separate the first cycle (%[0:n] -> %0 | [%1:n]) and
    /// replace %0 with `comp.go & %0`. (We do `comp.go & %0` rather than `%0` bc
    /// we want the clients to be able to assert `go` for n cycles and the
    /// component still works as expected).
    pub fn realize_schedule(
        &mut self,
        builder: &mut ir::Builder,
        static_component_interface: bool,
    ) -> (VecDeque<Vec<ir::Assignment<Nothing>>>, StaticFSM) {
        // First build the fsm we will use to realize the schedule.
        let fsm_object = StaticFSM::from_basic_info(
            self.num_states,
            FSMEncoding::Binary,
            builder,
        );

        // Instantiate the vecdeque.
        let mut res = VecDeque::new();
        for static_group in &mut self.static_groups {
            let mut static_group_ref = static_group.borrow_mut();
            // Separate the first cycle (if necessary) and then realize the
            // static timing guards (e.g., %[2:3] -> 2 <= fsm < 3).
            let group_assigns =
                std::mem::take(&mut static_group_ref.assignments);
            let static_assigns = if static_component_interface {
                group_assigns
                    .into_iter()
                    .map(|assign| {
                        if static_component_interface {
                            Self::handle_static_interface(
                                assign,
                                Rc::clone(&builder.component.signature),
                            )
                        } else {
                            assign
                        }
                    })
                    .collect_vec()
            } else {
                group_assigns
            };
            let mut assigns: Vec<ir::Assignment<Nothing>> = static_assigns
                .into_iter()
                .map(|static_assign| {
                    Self::make_assign_dyn(static_assign, &fsm_object, builder)
                })
                .collect();
            // For static components, we don't unconditionally start counting.
            // We must only start counting when `comp.go` is high.
            let fsm_incr_condition = if static_component_interface {
                let comp_sig = Rc::clone(&builder.component.signature);
                let g = guard!(comp_sig["go"]);
                Some(g)
            } else {
                None
            };
            // We need to add assignments that makes the FSM count to n.
            assigns.extend(fsm_object.count_to_n(
                builder,
                static_group_ref.get_latency() - 1,
                fsm_incr_condition,
            ));

            res.push_back(assigns);
        }
        (res, fsm_object)
    }

    // Takes in a static guard `guard`, and returns equivalent dynamic guard
    // The only thing that actually changes is the Guard::Info case
    // We need to turn static_timing to dynamic guards using `fsm`.
    // E.g.: %[2:3] gets turned into fsm.out >= 2 & fsm.out < 3
    // is_static_comp is necessary becasue it ...
    fn make_guard_dyn(
        guard: ir::Guard<ir::StaticTiming>,
        fsm_object: &StaticFSM,
        builder: &mut ir::Builder,
    ) -> Box<ir::Guard<Nothing>> {
        match guard {
            ir::Guard::Or(l, r) => Box::new(ir::Guard::Or(
                Self::make_guard_dyn(*l, fsm_object, builder),
                Self::make_guard_dyn(*r, fsm_object, builder),
            )),
            ir::Guard::And(l, r) => Box::new(ir::Guard::And(
                Self::make_guard_dyn(*l, fsm_object, builder),
                Self::make_guard_dyn(*r, fsm_object, builder),
            )),
            ir::Guard::Not(g) => Box::new(ir::Guard::Not(
                Self::make_guard_dyn(*g, fsm_object, builder),
            )),
            ir::Guard::CompOp(op, l, r) => {
                Box::new(ir::Guard::CompOp(op, l, r))
            }
            ir::Guard::Port(p) => Box::new(ir::Guard::Port(p)),
            ir::Guard::True => Box::new(ir::Guard::True),
            ir::Guard::Info(static_timing) => {
                fsm_object.query_between(builder, static_timing.get_interval())
            }
        }
    }

    // Takes in static assignment `assign` and returns a dynamic assignments
    // Mainly transforms the guards from %[2:3] -> fsm.out >= 2 & fsm.out <= 3
    fn make_assign_dyn(
        assign: ir::Assignment<ir::StaticTiming>,
        fsm_object: &StaticFSM,
        builder: &mut ir::Builder,
    ) -> ir::Assignment<Nothing> {
        ir::Assignment {
            src: assign.src,
            dst: assign.dst,
            attributes: assign.attributes,
            guard: Self::make_guard_dyn(*assign.guard, fsm_object, builder),
        }
    }

    // Looks recursively thru guard to transform %[0:n] into %0 | %[1:n].
    fn handle_static_interface_guard(
        guard: ir::Guard<ir::StaticTiming>,
        comp_sig: ir::RRC<ir::Cell>,
    ) -> ir::Guard<ir::StaticTiming> {
        match guard {
            ir::Guard::Info(st) => {
                let (beg, end) = st.get_interval();
                if beg == 0 {
                    // Replace %[0:n] -> (%0 & comp.go) | %[1:n]
                    // Cannot just do comp.go | %[1:n] because we want
                    // clients to be able to assert `comp.go` even after the first
                    // cycle w/o affecting correctness.
                    let first_cycle =
                        ir::Guard::Info(ir::StaticTiming::new((0, 1)));
                    let comp_go = guard!(comp_sig["go"]);
                    let first_and_go = ir::Guard::and(comp_go, first_cycle);
                    if end == 1 {
                        return first_and_go;
                    } else {
                        let after =
                            ir::Guard::Info(ir::StaticTiming::new((1, end)));
                        let cong = ir::Guard::or(first_and_go, after);
                        return cong;
                    }
                }
                guard
            }
            ir::Guard::And(l, r) => {
                let left = Self::handle_static_interface_guard(
                    *l,
                    Rc::clone(&comp_sig),
                );
                let right = Self::handle_static_interface_guard(*r, comp_sig);
                ir::Guard::and(left, right)
            }
            ir::Guard::Or(l, r) => {
                let left = Self::handle_static_interface_guard(
                    *l,
                    Rc::clone(&comp_sig),
                );
                let right = Self::handle_static_interface_guard(*r, comp_sig);
                ir::Guard::or(left, right)
            }
            ir::Guard::Not(g) => {
                let a = Self::handle_static_interface_guard(*g, comp_sig);
                ir::Guard::Not(Box::new(a))
            }
            _ => guard,
        }
    }

    // Looks recursively thru assignment's guard to %[0:n] into %0 | %[1:n].
    fn handle_static_interface(
        assign: ir::Assignment<ir::StaticTiming>,
        comp_sig: ir::RRC<ir::Cell>,
    ) -> ir::Assignment<ir::StaticTiming> {
        ir::Assignment {
            src: assign.src,
            dst: assign.dst,
            attributes: assign.attributes,
            guard: Box::new(Self::handle_static_interface_guard(
                *assign.guard,
                comp_sig,
            )),
        }
    }
}
