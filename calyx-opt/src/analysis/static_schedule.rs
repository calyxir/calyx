use crate::passes::math_utilities::get_bit_width_from;
use calyx_ir::{self as ir};
use calyx_ir::{build_assignments, Nothing};
use calyx_ir::{guard, structure};
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

#[derive(Debug)]
// Define an enum called Fruit
pub enum FSMEncoding {
    Binary,
    OneHot,
}

#[derive(Debug)]
struct StaticFSM {
    pub _num_states: u64,
    pub _encoding: FSMEncoding,
    pub static_component_interface: bool,
    bitwidth: u64,
    cell: ir::RRC<ir::Cell>,
}
impl StaticFSM {
    pub fn from_basic_info(
        num_states: u64,
        encoding: FSMEncoding,
        static_component_interface: bool,
        builder: &mut ir::Builder,
    ) -> Self {
        // First build the fsm we will use to realize the schedule.
        let fsm_size =
            get_bit_width_from(num_states + 1 /* represent 0..latency */);
        let fsm = builder.add_primitive("fsm", "std_reg", &[fsm_size]);
        StaticFSM {
            _num_states: num_states,
            _encoding: encoding,
            static_component_interface,
            bitwidth: fsm_size,
            cell: fsm,
        }
    }

    pub fn count_to_n(
        &self,
        builder: &mut ir::Builder,
        n: u64,
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
        if self.static_component_interface {
            // The requirements for components that need support the static
            // interface are slightly different.
            // We need to guard the FSM 0->1 transition with (fsm == 0 & comp.go)
            // The reason why we can't just gaurd with `comp.go` is because
            // we want clients to be able to assert `go` while the component
            // is executing without messing things up,
            // (even if asserting `go` is unnecessary.)
            let this: Rc<std::cell::RefCell<ir::Cell>> =
                Rc::clone(&builder.component.signature.clone());
            let this_go: ir::Guard<Nothing> = guard!(this["go"]);
            let first_state_guard: ir::Guard<Nothing> =
                guard!(fsm_cell["out"] == first_state["out"]);
            let go_and_first_state = ir::Guard::and(this_go, first_state_guard);
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
              // If fsm == 0 and comp.go is high, then we start an execution.
              fsm_cell["in"] = go_and_first_state ? const_one["out"];
              // If 1 < fsm < n - 1, then we unconditionally increment the fsm.
              fsm_cell["in"] = in_between_guard ? adder["out"];
              // If fsm == n -1 , then we reset the FSM.
              fsm_cell["in"] = final_state_guard ? first_state["out"];
              // Otherwise the FSM is not assigned to, so it defaults to 0.
              // If we want, we could add an explicit assignment here that sets it
              // to zero.
            )
            .to_vec()
        } else {
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
    }

    pub fn query_between(
        &self,
        builder: &mut ir::Builder,
        query: (u64, u64),
        comp_sig: Option<ir::RRC<ir::Cell>>,
    ) -> Box<ir::Guard<Nothing>> {
        let (beg, end) = query;
        let fsm_cell = Rc::clone(&self.cell);
        if self.static_component_interface && beg == 0 && end == 1 {
            // Replace `%0`` with `fsm == 0 & comp.go`.
            // The reason why we can't just gaurd with `comp.go` is because
            // we want clients to be able to assert `go` while the component
            // is executing without messing things up,
            // (even if asserting `go` is unnecessary.)
            let interval_const = builder.add_constant(0, self.bitwidth);
            let sig = comp_sig.unwrap();
            let g1 = guard!(sig["go"]);
            let g2 = guard!(fsm_cell["out"] == interval_const["out"]);
            let g = ir::Guard::And(Box::new(g1), Box::new(g2));
            return Box::new(g);
        }
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

    pub fn get_cell(&self) -> ir::RRC<ir::Cell> {
        Rc::clone(&self.cell)
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
    ) -> (VecDeque<Vec<ir::Assignment<Nothing>>>, ir::RRC<ir::Cell>) {
        // First build the fsm we will use to realize the schedule.
        let fsm_object = StaticFSM::from_basic_info(
            self.num_states,
            FSMEncoding::Binary,
            static_component_interface,
            builder,
        );

        // Instantiate the vecdeque.
        let mut res = VecDeque::new();
        for static_group in &mut self.static_groups {
            let mut static_group_ref = static_group.borrow_mut();
            // Separate the first cycle (if necessary) and then realize the
            // static timing guards (e.g., %[2:3] -> 2 <= fsm < 3).
            let mut assigns: Vec<ir::Assignment<Nothing>> = static_group_ref
                .assignments
                .drain(..)
                .map(|assign| {
                    if static_component_interface {
                        Self::separate_first_cycle_assign(assign)
                    } else {
                        assign
                    }
                })
                .map(|static_assign| {
                    Self::make_assign_dyn(
                        static_assign,
                        &fsm_object,
                        builder,
                        static_component_interface,
                        Some(Rc::clone(&builder.component.signature)),
                    )
                })
                .collect();
            assigns.extend(
                fsm_object
                    .count_to_n(builder, static_group_ref.get_latency() - 1),
            );

            res.push_back(assigns);
        }
        (res, fsm_object.get_cell())
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
        static_component_interface: bool,
        comp_sig: Option<ir::RRC<ir::Cell>>,
    ) -> Box<ir::Guard<Nothing>> {
        match guard {
            ir::Guard::Or(l, r) => Box::new(ir::Guard::Or(
                Self::make_guard_dyn(
                    *l,
                    fsm_object,
                    builder,
                    static_component_interface,
                    comp_sig.clone(),
                ),
                Self::make_guard_dyn(
                    *r,
                    fsm_object,
                    builder,
                    static_component_interface,
                    comp_sig,
                ),
            )),
            ir::Guard::And(l, r) => Box::new(ir::Guard::And(
                Self::make_guard_dyn(
                    *l,
                    fsm_object,
                    builder,
                    static_component_interface,
                    comp_sig.clone(),
                ),
                Self::make_guard_dyn(
                    *r,
                    fsm_object,
                    builder,
                    static_component_interface,
                    comp_sig,
                ),
            )),
            ir::Guard::Not(g) => {
                Box::new(ir::Guard::Not(Self::make_guard_dyn(
                    *g,
                    fsm_object,
                    builder,
                    static_component_interface,
                    comp_sig,
                )))
            }
            ir::Guard::CompOp(op, l, r) => {
                Box::new(ir::Guard::CompOp(op, l, r))
            }
            ir::Guard::Port(p) => Box::new(ir::Guard::Port(p)),
            ir::Guard::True => Box::new(ir::Guard::True),
            ir::Guard::Info(static_timing) => fsm_object.query_between(
                builder,
                static_timing.get_interval(),
                comp_sig,
            ),
        }
    }

    // Takes in static assignment `assign` and returns a dynamic assignments
    // Mainly transforms the guards from %[2:3] -> fsm.out >= 2 & fsm.out <= 3
    fn make_assign_dyn(
        assign: ir::Assignment<ir::StaticTiming>,
        fsm_object: &StaticFSM,
        builder: &mut ir::Builder,
        static_component_interface: bool,
        comp_sig: Option<ir::RRC<ir::Cell>>,
    ) -> ir::Assignment<Nothing> {
        ir::Assignment {
            src: assign.src,
            dst: assign.dst,
            attributes: assign.attributes,
            guard: Self::make_guard_dyn(
                *assign.guard,
                &fsm_object,
                builder,
                static_component_interface,
                comp_sig,
            ),
        }
    }

    // Looks recursively thru guard to transform %[0:n] into %0 | %[1:n].
    fn separate_first_cycle(
        guard: ir::Guard<ir::StaticTiming>,
    ) -> ir::Guard<ir::StaticTiming> {
        match guard {
            ir::Guard::Info(st) => {
                let (beg, end) = st.get_interval();
                if beg == 0 && end != 1 {
                    let first_cycle =
                        ir::Guard::Info(ir::StaticTiming::new((0, 1)));
                    let after =
                        ir::Guard::Info(ir::StaticTiming::new((1, end)));
                    let cong = ir::Guard::or(first_cycle, after);
                    return cong;
                }
                guard
            }
            ir::Guard::And(l, r) => {
                let left = Self::separate_first_cycle(*l);
                let right = Self::separate_first_cycle(*r);
                ir::Guard::and(left, right)
            }
            ir::Guard::Or(l, r) => {
                let left = Self::separate_first_cycle(*l);
                let right = Self::separate_first_cycle(*r);
                ir::Guard::or(left, right)
            }
            ir::Guard::Not(g) => {
                let a = Self::separate_first_cycle(*g);
                ir::Guard::Not(Box::new(a))
            }
            _ => guard,
        }
    }

    // Looks recursively thru assignment's guard to %[0:n] into %0 | %[1:n].
    fn separate_first_cycle_assign(
        assign: ir::Assignment<ir::StaticTiming>,
    ) -> ir::Assignment<ir::StaticTiming> {
        ir::Assignment {
            src: assign.src,
            dst: assign.dst,
            attributes: assign.attributes,
            guard: Box::new(Self::separate_first_cycle(*assign.guard)),
        }
    }
}
