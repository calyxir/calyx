use crate::passes::math_utilities::get_bit_width_from;
use calyx_ir::{self as ir};
use calyx_ir::{build_assignments, Nothing};
use calyx_ir::{guard, structure};
use core::panic;
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};
use std::ops::Not;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, Default)]
// Define an FSMEncoding Enum
pub enum FSMEncoding {
    #[default]
    Binary,
    OneHot,
}

#[derive(Debug)]
pub struct StaticFSM {
    fsm_cell: ir::RRC<ir::Cell>,
    encoding: FSMEncoding,
    // The fsm's bitwidth (this redundant information bc  we have `cell`)
    // but makes it easier if we easily have access to this.
    bitwidth: u64,
    // Mapping of queries from (u64, u64) -> Port
    queries: HashMap<(u64, u64), ir::RRC<ir::Port>>,
}
impl StaticFSM {
    // Builds a static_fsm from: num_states and encoding type.
    pub fn from_basic_info(
        num_states: u64,
        encoding: FSMEncoding,
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

        StaticFSM {
            encoding,
            fsm_cell: register,
            bitwidth: fsm_size,
            queries: HashMap::new(),
        }
    }

    // Builds an incrementer, and returns the assignments and incrementer cell itself.
    // assignments are:
    // incrementer.left = fsm.out; incrementer.right = 1;
    // cell is:
    // incrementer
    pub fn build_incrementer(
        &self,
        builder: &mut ir::Builder,
    ) -> (Vec<ir::Assignment<Nothing>>, ir::RRC<ir::Cell>) {
        let fsm_cell = Rc::clone(&self.fsm_cell);
        // For OHE, the "adder" can just be a shifter.
        // For OHE the first_state = 1 rather than 0.
        // Final state is encoded differently for OHE vs. Binary
        let adder = match self.encoding {
            FSMEncoding::Binary => {
                builder.add_primitive("adder", "std_add", &[self.bitwidth])
            }
            FSMEncoding::OneHot => {
                builder.add_primitive("lsh", "std_lsh", &[self.bitwidth])
            }
        };
        let const_one = builder.add_constant(1, self.bitwidth);
        let incr_assigns = build_assignments!(
          builder;
          // increments the fsm
          adder["left"] = ? fsm_cell["out"];
          adder["right"] = ? const_one["out"];
        )
        .to_vec();
        (incr_assigns, adder)
    }

    // Returns the assignments that conditionally increment the fsm,
    // but only if guard is true.
    pub fn conditional_increment(
        &self,
        guard: ir::Guard<Nothing>,
        adder: ir::RRC<ir::Cell>,
        builder: &mut ir::Builder,
    ) -> Vec<ir::Assignment<Nothing>> {
        let fsm_cell = Rc::clone(&self.fsm_cell);
        let signal_on = builder.add_constant(1, 1);
        let assigns = build_assignments!(
          builder;
          // increments the fsm
          fsm_cell["in"] = guard ? adder["out"];
          fsm_cell["write_en"] = guard ? signal_on["out"];
        )
        .to_vec();
        assigns
    }

    // Returns the assignments that conditionally resets the fsm to 0,
    // but only if guard is true.
    pub fn conditional_reset(
        &self,
        guard: ir::Guard<Nothing>,
        builder: &mut ir::Builder,
    ) -> Vec<ir::Assignment<Nothing>> {
        let fsm_cell = Rc::clone(&self.fsm_cell);
        let signal_on = builder.add_constant(1, 1);
        let const_0 = builder.add_constant(0, self.bitwidth);
        let assigns = build_assignments!(
          builder;
          fsm_cell["in"] = guard ? const_0["out"];
          fsm_cell["write_en"] = guard ? signal_on["out"];
        )
        .to_vec();
        assigns
    }

    // Returns a guard that takes a (beg, end) `query`, and returns the equivalent
    // guard to `beg <= fsm.out < end`.
    pub fn query_between(
        &mut self,
        builder: &mut ir::Builder,
        query: (u64, u64),
    ) -> Box<ir::Guard<Nothing>> {
        let (beg, end) = query;
        // Querying OHE is easy, since we already have `self.get_one_hot_query()`
        let fsm_cell = Rc::clone(&self.fsm_cell);
        if matches!(self.encoding, FSMEncoding::OneHot) {
            let g = self.get_one_hot_query(fsm_cell, (beg, end), builder);
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
        self.fsm_cell.borrow().name()
    }

    // Return the bitwidth of an FSM object
    pub fn get_bitwidth(&self) -> u64 {
        self.bitwidth
    }
}

pub enum FSMTree {
    Tree(Tree),
    Par(ParTree),
}

impl FSMTree {
    pub fn count_to_n(&mut self, builder: &mut ir::Builder) {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.count_to_n(builder),
            FSMTree::Par(_) => panic!(""),
        }
    }

    pub fn realize(
        &mut self,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        fsm_info_map: &mut HashMap<ir::Id, ir::RRC<StaticFSM>>,
        group_rewrites: &mut HashMap<ir::Canonical, ir::RRC<ir::Port>>,
        builder: &mut ir::Builder,
    ) {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.realize(
                static_groups,
                reset_early_map,
                fsm_info_map,
                group_rewrites,
                builder,
            ),
            FSMTree::Par(_) => panic!(""),
        }
    }

    fn get_final_state(
        &mut self,
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.get_final_state(builder),
            FSMTree::Par(_) => panic!(""),
        }
    }

    pub fn get_group_name(&self) -> ir::Id {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.root.0,
            FSMTree::Par(par_struct) => panic!(""),
        }
    }

    fn get_latency(&self) -> u64 {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.latency,
            FSMTree::Par(par_struct) => par_struct.latency,
        }
    }
}

/// Helpful for translating queries, e.g., %[2:20].
/// Because of the tree structure,
/// this no longer is always equivalent to 2 <= fsm < 20;
#[derive(Debug)]
pub enum StateType {
    Delay(u64),
    Offload(u64),
}
pub struct Tree {
    pub latency: u64,
    pub num_repeats: u64,
    pub root: (ir::Id, Vec<ir::Assignment<Nothing>>),
    pub delay_map: BTreeMap<(u64, u64), StateType>,
    pub children: Vec<(FSMTree, (u64, u64))>,
    pub fsm_cell: Option<ir::RRC<StaticFSM>>,
    pub iter_count_cell: Option<ir::RRC<StaticFSM>>,
    pub incrementer: Option<ir::RRC<ir::Cell>>,
}

impl Tree {
    // Get the tree's final state (i.e., the state of the tree during its final cycle).
    fn get_final_state(
        &mut self,
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        let fsm_final_state = match &mut self.fsm_cell {
            None => {
                // If there is no FSM, then we know latency has to be 1.
                assert!(self.latency == 1);
                Box::new(ir::Guard::True)
            }
            Some(static_fsm_cell) => {
                // If there is an FSM, we check whether during its final state,
                // it is offloading or incrementing.
                let static_fsm = Rc::clone(&static_fsm_cell);

                if let Some((child, (beg_interval, end_interval))) =
                    self.children.last_mut()
                {
                    // You have to clone these earlier to avoid borrow checker nonsense.
                    // XXX(Caleb): think of better solution.
                    let beg_interval_clone = *beg_interval;
                    let end_interval_clone = *end_interval;
                    if *end_interval == self.latency {
                        let final_child_state =
                            Box::new(child.get_final_state(builder));
                        let final_parent_state =
                            static_fsm.borrow_mut().query_between(
                                builder,
                                self.translate_query((
                                    beg_interval_clone,
                                    end_interval_clone,
                                )),
                            );
                        return final_child_state.and(*final_parent_state);
                    }
                }
                // Otherwise, can just do a normal query.
                let x = static_fsm.borrow_mut().query_between(
                    builder,
                    self.translate_query((self.latency - 1, self.latency)),
                );
                x
            }
        };
        let counter_final_state = match &mut self.iter_count_cell {
            None => {
                assert!(self.num_repeats == 1);
                Box::new(ir::Guard::True)
            }
            Some(static_fsm) => static_fsm.borrow_mut().query_between(
                builder,
                (self.num_repeats - 1, self.num_repeats),
            ),
        };
        ir::Guard::And(fsm_final_state, counter_final_state)
    }

    fn extract_fsm_cell(&mut self) -> ir::RRC<StaticFSM> {
        let x = self.fsm_cell.as_ref().expect("field was None");
        Rc::clone(x)
    }

    fn count_to_n(&mut self, builder: &mut ir::Builder) {
        // offload_states are the fsm_states that last multiple cycles
        // because they offload computations to children.
        let mut offload_states = vec![];
        // Need to calculate offload_states. %[500:600] might not be at fsm=500
        // if there were previous states that were offloaded.
        let mut cur_delay = 0;
        for (_, (beg, end)) in &self.children {
            offload_states.push((beg - cur_delay, (end - beg)));
            cur_delay += end - beg;
        }
        let num_states = self.latency - cur_delay;

        let mut res_vec: Vec<ir::Assignment<Nothing>> = Vec::new();
        // Parent FSM for the "root" of the tree.
        let mut parent_fsm = StaticFSM::from_basic_info(
            num_states,
            FSMEncoding::Binary, // XXX(Caleb): change this
            builder,
        );
        let (adder_asssigns, adder) = parent_fsm.build_incrementer(builder);

        // Now handle the children, i.e., offload states.
        let mut offload_state_incrs = Vec::new();
        for (i, (child, _)) in self.children.iter_mut().enumerate() {
            // Let the child count to n.
            child.count_to_n(builder);

            // Increment parent when child is in final state.
            // e.g., fsm.in = fsm == 4 && child_fsm_in_final_state ? fsm + 1;
            // fsm.write_en = ... // same assignments
            let (child_state, _) = offload_states[i];
            let in_child_state = parent_fsm
                .query_between(builder, (child_state, child_state + 1));
            let final_child_state = child.get_final_state(builder);
            let parent_fsm_incr = parent_fsm.conditional_increment(
                ir::Guard::And(in_child_state, Box::new(final_child_state)),
                Rc::clone(&adder),
                builder,
            );
            offload_state_incrs.extend(parent_fsm_incr);
        }

        // Getting final state for the fsm.
        let final_state_guard = if let Some((child, (_, end_interval))) =
            self.children.last_mut()
        {
            if *end_interval == self.latency {
                child.get_final_state(builder) // XXX(Caleb): need to fix this possibly
            } else {
                *parent_fsm.query_between(
                    builder,
                    (self.latency - 1 - cur_delay, self.latency - cur_delay),
                )
            }
        } else {
            *parent_fsm.query_between(
                builder,
                (self.latency - 1 - cur_delay, self.latency - cur_delay),
            )
        };
        let not_final_state = final_state_guard.clone().not();

        offload_state_incrs.iter_mut().for_each(|assign| {
            assign.guard.update(|g| g.and(not_final_state.clone()));
        });
        res_vec.extend(offload_state_incrs);

        // offload_state_guard initialized to false.
        let mut offload_state_guard: ir::Guard<Nothing> =
            ir::Guard::Not(Box::new(ir::Guard::True));
        for (offload_state, _) in &offload_states {
            // Creating a guard that checks whether the parent fsm is
            // in an offload state.
            offload_state_guard.update(|g| {
                g.or(*parent_fsm.query_between(
                    builder,
                    (*offload_state, offload_state + 1),
                ))
            });
        }

        // Increment when fsm is not in an offload state.
        let not_offload_state = offload_state_guard.not();

        res_vec.extend(adder_asssigns);
        res_vec.extend(parent_fsm.conditional_increment(
            not_offload_state.and(not_final_state.clone()),
            Rc::clone(&adder),
            builder,
        ));

        res_vec.extend(
            parent_fsm.conditional_reset(final_state_guard.clone(), builder),
        );

        // Handle num_repeats.
        if self.num_repeats != 1 {
            let mut repeat_fsm = StaticFSM::from_basic_info(
                self.num_repeats,
                FSMEncoding::Binary, // XXX(Caleb): change this
                builder,
            );
            let (adder_asssigns, adder) = repeat_fsm.build_incrementer(builder);
            let final_repeat_state = *repeat_fsm.query_between(
                builder,
                (self.num_repeats - 1, self.num_repeats),
            );
            let not_final_repeat_state = final_repeat_state.clone().not();
            res_vec.extend(adder_asssigns);
            res_vec.extend(repeat_fsm.conditional_increment(
                final_state_guard.clone().and(not_final_repeat_state),
                adder,
                builder,
            ));

            res_vec.extend(repeat_fsm.conditional_reset(
                final_state_guard.clone().and(final_repeat_state),
                builder,
            ));
            self.iter_count_cell = Some(ir::rrc(repeat_fsm));
        }

        self.fsm_cell = Some(ir::rrc(parent_fsm));
        let (_, root_asgns) = &mut self.root;
        root_asgns.extend(res_vec);
    }

    fn translate_query(&self, query: (u64, u64)) -> (u64, u64) {
        let (beg_query, end_query) = query;
        for ((beg, end), state_type) in &self.delay_map {
            if (beg_query >= *beg) && (end_query <= *end) {
                match state_type {
                    StateType::Delay(delay) => {
                        return (beg_query - delay, end_query - delay);
                    }
                    StateType::Offload(state) => {
                        assert!((*beg == beg_query) && *end == end_query);
                        return (*state, state + 1);
                    }
                }
            }
        }
        panic!("")
    }

    fn realize(
        &mut self,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        fsm_info_map: &mut HashMap<ir::Id, ir::RRC<StaticFSM>>,
        group_rewrites: &mut HashMap<ir::Canonical, ir::RRC<ir::Port>>,
        builder: &mut ir::Builder,
    ) {
        let static_group = Rc::clone(
            &static_groups
                .iter()
                .find(|sgroup| sgroup.borrow().name() == self.root.0)
                .unwrap(),
        );
        // Create the dynamic "early reset group" that will replace the static group.
        let static_group_name = static_group.borrow().name();
        let mut early_reset_name = static_group_name.to_string();
        early_reset_name.insert_str(0, "early_reset_");
        let early_reset_group = builder.add_group(early_reset_name);
        let fsm_ref = self.extract_fsm_cell();
        let mut assigns =
            std::mem::take(&mut static_group.borrow().assignments.clone())
                .into_iter()
                .map(|assign| {
                    FSMTree::make_assign_dyn(
                        assign,
                        Rc::clone(&fsm_ref),
                        builder,
                        &self.delay_map,
                    )
                })
                .collect_vec();

        // Add assignment `group[done] = ud.out`` to the new group.
        structure!( builder; let ud = prim undef(1););
        let early_reset_done_assign = build_assignments!(
          builder;
          early_reset_group["done"] = ? ud["out"];
        );
        assigns.extend(early_reset_done_assign);
        assigns.extend(std::mem::take(&mut self.root.1));

        early_reset_group.borrow_mut().assignments = assigns;
        early_reset_group.borrow_mut().attributes =
            static_group.borrow().attributes.clone();

        // Now we have to update the fields with a bunch of information.
        // This makes it easier when we have to build wrappers, rewrite ports, etc.

        // Map the static group name -> early reset group name.
        // This is helpful for rewriting control
        reset_early_map
            .insert(static_group_name, early_reset_group.borrow().name());
        // self.group_rewrite_map helps write static_group[go] to early_reset_group[go]
        // Technically we could do this w/ early_reset_map but is easier w/
        // group_rewrite, which is explicitly of type `PortRewriterMap`
        group_rewrites.insert(
            ir::Canonical::new(static_group_name, ir::Id::from("go")),
            early_reset_group.borrow().find("go").unwrap_or_else(|| {
                unreachable!(
                    "group {} has no go port",
                    early_reset_group.borrow().name()
                )
            }),
        );
        fsm_info_map
            .insert(early_reset_group.borrow().name(), Rc::clone(&fsm_ref));

        self.children.iter_mut().for_each(|(child, _)| {
            child.realize(
                static_groups,
                reset_early_map,
                fsm_info_map,
                group_rewrites,
                builder,
            )
        })
    }
}
pub struct ParTree {
    pub latency: u64,
    pub threads: Vec<(FSMTree, (u64, u64))>,
    pub num_repeats: u64,
}

impl FSMTree {
    // Takes in a static guard `guard`, and returns equivalent dynamic guard
    // The only thing that actually changes is the Guard::Info case
    // We need to turn static_timing to dynamic guards using `fsm`.
    // E.g.: %[2:3] gets turned into fsm.out >= 2 & fsm.out < 3
    fn make_guard_dyn(
        guard: ir::Guard<ir::StaticTiming>,
        fsm_object: ir::RRC<StaticFSM>,
        builder: &mut ir::Builder,
        delay_map: &BTreeMap<(u64, u64), StateType>,
    ) -> Box<ir::Guard<Nothing>> {
        match guard {
            ir::Guard::Or(l, r) => Box::new(ir::Guard::Or(
                Self::make_guard_dyn(
                    *l,
                    Rc::clone(&fsm_object),
                    builder,
                    delay_map,
                ),
                Self::make_guard_dyn(
                    *r,
                    Rc::clone(&fsm_object),
                    builder,
                    delay_map,
                ),
            )),
            ir::Guard::And(l, r) => Box::new(ir::Guard::And(
                Self::make_guard_dyn(
                    *l,
                    Rc::clone(&fsm_object),
                    builder,
                    delay_map,
                ),
                Self::make_guard_dyn(
                    *r,
                    Rc::clone(&fsm_object),
                    builder,
                    delay_map,
                ),
            )),
            ir::Guard::Not(g) => Box::new(ir::Guard::Not(
                Self::make_guard_dyn(*g, fsm_object, builder, delay_map),
            )),
            ir::Guard::CompOp(op, l, r) => {
                Box::new(ir::Guard::CompOp(op, l, r))
            }
            ir::Guard::Port(p) => Box::new(ir::Guard::Port(p)),
            ir::Guard::True => Box::new(ir::Guard::True),
            ir::Guard::Info(static_timing) => {
                let (beg_target, end_target) = static_timing.get_interval();
                for ((beg, end), state_type) in delay_map {
                    if (beg_target >= *beg) && (end_target <= *end) {
                        match state_type {
                            StateType::Delay(delay) => {
                                return fsm_object.borrow_mut().query_between(
                                    builder,
                                    (beg_target - delay, end_target - delay),
                                );
                            }
                            StateType::Offload(state) => {
                                assert!(
                                    (*beg == beg_target) && *end == end_target
                                );
                                return fsm_object.borrow_mut().query_between(
                                    builder,
                                    (*state, state + 1),
                                );
                            }
                        }
                    }
                }
                panic!("");
            }
        }
    }

    // Takes in static assignment `assign` and returns a dynamic assignments
    // Mainly transforms the guards from %[2:3] -> fsm.out >= 2 & fsm.out <= 3
    pub fn make_assign_dyn(
        assign: ir::Assignment<ir::StaticTiming>,
        fsm_object: ir::RRC<StaticFSM>,
        builder: &mut ir::Builder,
        delay_map: &BTreeMap<(u64, u64), StateType>,
    ) -> ir::Assignment<Nothing> {
        ir::Assignment {
            src: assign.src,
            dst: assign.dst,
            attributes: assign.attributes,
            guard: Self::make_guard_dyn(
                *assign.guard,
                fsm_object,
                builder,
                &delay_map,
            ),
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
