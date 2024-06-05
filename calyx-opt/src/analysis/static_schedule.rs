use crate::passes::math_utilities::get_bit_width_from;
use calyx_ir::{self as ir};
use calyx_ir::{build_assignments, Nothing};
use calyx_ir::{guard, structure};
use ir::Guard;
use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Not;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, Default)]
// Define an FSMEncoding Enum
enum FSMEncoding {
    #[default]
    Binary,
    OneHot,
}

#[derive(Debug)]
enum FSMImplementationSpec {
    Single,
    // How many duplicates
    _Duplicate(u64),
    // How many times to split
    _Split(u64),
}

#[derive(Debug)]
// Define an enum called FSMType
enum FSMImplementation {
    // Default option: just a single register
    Single(ir::RRC<ir::Cell>),
    // Duplicate the register to reduce fanout when querying
    // (all FSMs in this vec still have all of the states)
    _Duplicate(Vec<ir::RRC<ir::Cell>>),
    // Split the FSM to reduce fanout when querying.
    // (the FSMs partition the states exactly).
    // Each FSM has fewer bits but I suspect the logic might be more complicated.
    _Split(Vec<ir::RRC<ir::Cell>>),
}

impl FSMImplementation {
    fn get_single_cell(&self) -> ir::RRC<ir::Cell> {
        match self {
            FSMImplementation::Single(cell) => Rc::clone(cell),
            _ => unreachable!(
                "called `get_single_cell()` on non-single FSM implementation "
            ),
        }
    }
}

#[derive(Debug)]
pub struct StaticFSM {
    encoding: FSMEncoding,
    // The fsm's bitwidth (this redundant information bc  we have `cell`)
    // but makes it easier if we easily have access to this.
    bitwidth: u64,
    // The actual register(s) used to implement the FSM
    implementation: FSMImplementation,
    // Mapping of queries from (u64, u64) -> Port
    queries: HashMap<(u64, u64), ir::RRC<ir::Port>>,
}
impl StaticFSM {
    // Builds a static_fsm from: num_states and encoding type.
    fn from_basic_info(
        num_states: u64,
        encoding: FSMEncoding,
        _implementation: FSMImplementationSpec,
        builder: &mut ir::Builder,
    ) -> Self {
        // Determine number of bits needed in the register.
        let fsm_size = match encoding {
            /* represent 0..latency */
            FSMEncoding::Binary => get_bit_width_from(num_states + 1),
            FSMEncoding::OneHot => num_states,
        };
        // OHE needs an initial value of 1.
        let register = match encoding {
            FSMEncoding::Binary => {
                builder.add_primitive("fsm", "std_reg", &[fsm_size])
            }
            FSMEncoding::OneHot => {
                builder.add_primitive("fsm", "init_one_reg", &[fsm_size])
            }
        };
        let fsm = FSMImplementation::Single(register);

        StaticFSM {
            encoding,
            bitwidth: fsm_size,
            implementation: fsm,
            queries: HashMap::new(),
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
    // (IMPORTANT WEIRD PRECONDITION): if `incr_cond` is Some(_), we assume n > 0.
    pub fn count_to_n(
        &mut self,
        builder: &mut ir::Builder,
        n: u64,
        incr_condition: Option<Guard<Nothing>>,
    ) -> Vec<ir::Assignment<Nothing>> {
        {
            assert!(matches!(
                self.implementation,
                FSMImplementation::Single(_)
            ));
            let fsm_cell: Rc<std::cell::RefCell<ir::Cell>> =
                self.implementation.get_single_cell();
            // For OHE, the "adder" can just be a shifter.
            // For OHE the first_state = 1 rather than 0.
            // Final state is encoded differently for OHE vs. Binary
            let (adder, first_state, final_state_guard) = match self.encoding {
                FSMEncoding::Binary => (
                    builder.add_primitive("adder", "std_add", &[self.bitwidth]),
                    builder.add_constant(0, self.bitwidth),
                    {
                        let const_n = builder.add_constant(n, self.bitwidth);
                        let g = guard!(fsm_cell["out"] == const_n["out"]);
                        g
                    },
                ),
                FSMEncoding::OneHot => (
                    builder.add_primitive("lsh", "std_lsh", &[self.bitwidth]),
                    builder.add_constant(1, self.bitwidth),
                    self.get_one_hot_query(
                        Rc::clone(&fsm_cell),
                        (n, n + 1),
                        builder,
                    ),
                ),
            };
            structure!( builder;
                let signal_on = constant(1,1);
                let const_one = constant(1, self.bitwidth);
            );
            let not_final_state_guard =
                ir::Guard::Not(Box::new(final_state_guard.clone()));
            match incr_condition {
                None => {
                    // Unconditionally increment FSM.
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
                    // Only start incrementing when FSM == first_state and
                    // conditiona_guard is true.
                    // After that, we can unconditionally increment.
                    let first_state_guard = match self.encoding {
                        FSMEncoding::Binary => {
                            let g =
                                guard!(fsm_cell["out"] == first_state["out"]);
                            g
                        }
                        // This is better than checking if FSM == first_state
                        // be this is only checking a single bit.
                        FSMEncoding::OneHot => self.get_one_hot_query(
                            Rc::clone(&fsm_cell),
                            (0, 1),
                            builder,
                        ),
                    };
                    let not_first_state: ir::Guard<Nothing> =
                        ir::Guard::Not(Box::new(first_state_guard.clone()));
                    let cond_and_first_state = ir::Guard::and(
                        condition_guard.clone(),
                        first_state_guard.clone(),
                    );
                    let not_cond_and_first_state =
                        ir::Guard::not(condition_guard.clone())
                            .and(first_state_guard);
                    let in_between_guard =
                        ir::Guard::and(not_first_state, not_final_state_guard);
                    let my_assigns = build_assignments!(
                      builder;
                      // Incrementsthe fsm
                      adder["left"] = ? fsm_cell["out"];
                      adder["right"] = ? const_one["out"];
                      // Always write into fsm.
                      fsm_cell["write_en"] = ? signal_on["out"];
                      // If fsm == first_state and cond is high, then we start an execution.
                      fsm_cell["in"] = cond_and_first_state ? adder["out"];
                      // If first_state < fsm < n, then we unconditionally increment the fsm.
                      fsm_cell["in"] = in_between_guard ? adder["out"];
                      // If fsm == n, then we reset the FSM.
                      fsm_cell["in"] = final_state_guard ? first_state["out"];
                      // Otherwise we set the FSM equal to first_state.
                      fsm_cell["in"] = not_cond_and_first_state ? first_state["out"];
                    );
                    my_assigns.to_vec()
                }
            }
        }
    }

    // Returns a guard that takes a (beg, end) `query`, and returns the equivalent
    // guard to `beg <= fsm.out < end`.
    pub fn query_between(
        &mut self,
        builder: &mut ir::Builder,
        query: (u64, u64),
    ) -> Box<ir::Guard<Nothing>> {
        assert!(matches!(self.implementation, FSMImplementation::Single(_)));

        let (beg, end) = query;
        if matches!(self.encoding, FSMEncoding::OneHot) {
            // Querying OHE is easy, since we already have `self.get_one_hot_query()`
            let fsm_cell = self.implementation.get_single_cell();
            let g = self.get_one_hot_query(fsm_cell, (beg, end), builder);
            return Box::new(g);
        }

        let fsm_cell = self.implementation.get_single_cell();
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

    // Given a one-hot query, it will return a guard corresponding to that query.
    // If it has already built the query (i.e., added the wires/continuous assigments),
    // it just uses the same port.
    // Otherwise it will build the query.
    fn get_one_hot_query(
        &mut self,
        fsm_cell: ir::RRC<ir::Cell>,
        (lb, ub): (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        match self.queries.get(&(lb, ub)) {
            None => {
                let port = Self::build_one_hot_query(
                    Rc::clone(&fsm_cell),
                    self.bitwidth,
                    (lb, ub),
                    builder,
                );
                self.queries.insert((lb, ub), Rc::clone(&port));
                ir::Guard::port(port)
            }
            Some(port) => ir::Guard::port(Rc::clone(port)),
        }
    }

    // Given a (lb, ub) query, and an fsm (and for convenience, a bitwidth),
    // Returns a `port`: port is a `wire.out`, where `wire` holds
    // whether or not the query is true, i.e., whether the FSM really is
    // between [lb, ub).
    fn build_one_hot_query(
        fsm_cell: ir::RRC<ir::Cell>,
        fsm_bitwidth: u64,
        (lb, ub): (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::RRC<ir::Port> {
        // The wire that holds the query
        let formatted_name = format!("bw_{}_{}", lb, ub);
        let wire: ir::RRC<ir::Cell> =
            builder.add_primitive(formatted_name, "std_wire", &[1]);
        let wire_out = wire.borrow().get("out");

        // Continuous assignments to check the FSM
        let assigns = {
            let in_width = fsm_bitwidth;
            // Since 00...00 is the initial state, we need to check lb-1.
            let start_index = lb;
            // Since verilog slices are inclusive.
            let end_index = ub - 1;
            let out_width = ub - lb; // == (end_index - start_index + 1)
            structure!(builder;
                let slicer = prim std_bit_slice(in_width, start_index, end_index, out_width);
                let const_slice_0 = constant(0, out_width);
                let signal_on = constant(1,1);
            );
            let slicer_neq_0 = guard!(slicer["out"] != const_slice_0["out"]);
            // Extend the continuous assignmments to include this particular query for FSM state;
            let my_assigns = build_assignments!(builder;
                slicer["in"] = ? fsm_cell["out"];
                wire["in"] = slicer_neq_0 ? signal_on["out"];
            );
            my_assigns.to_vec()
        };
        builder.add_continuous_assignments(assigns);
        wire_out
    }

    // Return a unique id (i.e., get_unique_id for each FSM in the same component
    // will be different).
    pub fn get_unique_id(&self) -> ir::Id {
        assert!(matches!(self.implementation, FSMImplementation::Single(_)));
        self.implementation.get_single_cell().borrow().name()
    }

    // Return the bitwidth of an FSM object
    pub fn get_bitwidth(&self) -> u64 {
        assert!(matches!(self.implementation, FSMImplementation::Single(_)));
        self.bitwidth
    }
}

/// Represents a static schedule.
#[derive(Debug, Default)]
pub struct StaticSchedule {
    /// Number of states for the FSM
    /// (this is just the latency of the static island-- or that of the largest
    /// static island, if there are multiple islands)
    num_states: u64,
    /// The queries that the FSM needs to support.
    /// E.g., `lhs = %[2:3] ? rhs` corresponds to (2,3).
    queries: HashSet<(u64, u64)>,
    /// Encoding type for the FSM
    encoding: FSMEncoding,
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
        // iter().any() or iter().all() should both work, since our coloring
        // algorithm inserts conflicts if the @one_hot attribute doesn't match.
        schedule.encoding =
            if schedule.static_groups.iter().any(|sgroup| {
                sgroup.borrow().attributes.has(ir::BoolAttr::OneHot)
            }) {
                FSMEncoding::OneHot
            } else {
                FSMEncoding::Binary
            };
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
    /// If `self.static_groups = vec![group1, group2, group3, ...]``
    /// Then `realize_schedule()` returns vecdeque![a1, a2, a3]
    /// Where a1 are the assignments for group1, a2 are the assignments
    /// to group2, etc.
    /// It also returns the StaticFSM object.
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
        let mut fsm_object = StaticFSM::from_basic_info(
            self.num_states,
            self.encoding,
            FSMImplementationSpec::Single,
            builder,
        );

        // Instantiate the vecdeque.
        let mut res = VecDeque::new();
        for static_group in &mut self.static_groups {
            let mut static_group_ref = static_group.borrow_mut();
            // Separate the first cycle (if necessary) and then realize the
            // static timing guards (e.g., %[2:3] -> 2 <= fsm < 3).
            let group_assigns =
                static_group_ref.assignments.drain(..).collect_vec();
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
                    Self::make_assign_dyn(
                        static_assign,
                        &mut fsm_object,
                        builder,
                    )
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
        fsm_object: &mut StaticFSM,
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
        fsm_object: &mut StaticFSM,
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
