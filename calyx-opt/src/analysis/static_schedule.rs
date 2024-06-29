use crate::passes::math_utilities::get_bit_width_from;
use calyx_ir::{self as ir};
use calyx_ir::{build_assignments, Nothing};
use calyx_ir::{guard, structure};
use core::panic;
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};
use std::ops::Not;
use std::rc::Rc;

use super::GraphColoring;

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
    pub fn instantiate_fsms(
        &mut self,
        builder: &mut ir::Builder,
        coloring: &HashMap<ir::Id, ir::Id>,
        colors_to_max_values: &HashMap<ir::Id, (u64, u64)>,
        colors_to_fsm: &mut HashMap<
            ir::Id,
            (Option<ir::RRC<StaticFSM>>, Option<ir::RRC<StaticFSM>>),
        >,
    ) {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.instantiate_fsms(
                builder,
                coloring,
                colors_to_max_values,
                colors_to_fsm,
            ),
            FSMTree::Par(par_struct) => par_struct.instantiate_fsms(
                builder,
                coloring,
                colors_to_max_values,
                colors_to_fsm,
            ),
        }
    }

    pub fn count_to_n(
        &mut self,
        builder: &mut ir::Builder,
        incr_start_cond: Option<ir::Guard<Nothing>>,
    ) {
        match self {
            FSMTree::Tree(tree_struct) => {
                tree_struct.count_to_n(builder, incr_start_cond)
            }
            FSMTree::Par(par_struct) => {
                par_struct.count_to_n(builder, incr_start_cond)
            }
        }
    }

    pub fn realize(
        &mut self,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        fsm_info_map: &mut HashMap<
            ir::Id,
            (ir::Id, ir::Guard<Nothing>, ir::Guard<Nothing>),
        >,
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
            FSMTree::Par(par_struct) => par_struct.realize(
                static_groups,
                reset_early_map,
                fsm_info_map,
                group_rewrites,
                builder,
            ),
        }
    }

    pub fn query_between(
        &mut self,
        query: (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        match self {
            FSMTree::Tree(tree_struct) => {
                tree_struct.query_between(query, builder)
            }
            FSMTree::Par(par_struct) => {
                par_struct.query_between(query, builder)
            }
        }
    }

    pub fn transform_static_assigns(
        &mut self,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        group_rewrites: &mut HashMap<ir::Canonical, ir::RRC<ir::Port>>,
        builder: &mut ir::Builder,
    ) {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.transform_static_assigns(
                static_groups,
                reset_early_map,
                group_rewrites,
                builder,
            ),
            FSMTree::Par(par_struct) => par_struct.transform_static_assigns(
                static_groups,
                reset_early_map,
                group_rewrites,
                builder,
            ),
        }
    }

    pub fn take_root_assigns(&mut self) -> Vec<ir::Assignment<Nothing>> {
        match self {
            FSMTree::Tree(tree_struct) => {
                std::mem::take(&mut tree_struct.root.1)
            }
            FSMTree::Par(_) => {
                panic!("Cannot take root assignments of par_struct")
            }
        }
    }

    pub fn get_root_name(&mut self) -> ir::Id {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.root.0,
            FSMTree::Par(_) => {
                panic!("Cannot take root name of par_struct")
            }
        }
    }

    pub fn get_group_name(&self) -> ir::Id {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.root.0,
            FSMTree::Par(par_struct) => par_struct.group_name,
        }
    }

    pub fn get_latency(&self) -> u64 {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.latency,
            FSMTree::Par(par_struct) => par_struct.latency,
        }
    }

    pub fn get_id(&self) -> ir::Id {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.root.0,
            FSMTree::Par(par_struct) => par_struct.group_name,
        }
    }

    pub fn get_children(&mut self) -> &mut Vec<(FSMTree, (u64, u64))> {
        match self {
            FSMTree::Tree(tree_struct) => &mut tree_struct.children,
            FSMTree::Par(par_struct) => &mut par_struct.threads,
        }
    }

    fn get_num_repeats(&self) -> u64 {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.num_repeats,
            FSMTree::Par(par_struct) => par_struct.num_repeats,
        }
    }

    pub fn get_all_nodes(&self) -> Vec<ir::Id> {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.get_all_nodes(),
            FSMTree::Par(par_struct) => par_struct.get_all_nodes(),
        }
    }

    pub fn add_conflicts(&self, conflict_graph: &mut GraphColoring<ir::Id>) {
        match self {
            FSMTree::Tree(tree_struct) => {
                tree_struct.add_conflicts(conflict_graph)
            }
            FSMTree::Par(par_struct) => {
                par_struct.add_conflicts(conflict_graph)
            }
        }
    }

    pub fn get_max_value<F>(&self, name: &ir::Id, f: &F) -> u64
    where
        F: Fn(&Tree) -> u64,
    {
        match self {
            FSMTree::Tree(tree_struct) => tree_struct.get_max_value(name, f),
            FSMTree::Par(par_struct) => par_struct.get_max_value(name, f),
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
    pub num_states: u64,
    pub root: (ir::Id, Vec<ir::Assignment<Nothing>>),
    pub delay_map: BTreeMap<(u64, u64), StateType>,
    pub children: Vec<(FSMTree, (u64, u64))>,
    pub fsm_cell: Option<ir::RRC<StaticFSM>>,
    pub iter_count_cell: Option<ir::RRC<StaticFSM>>,
    pub incrementer: Option<ir::RRC<ir::Cell>>,
}

impl Tree {
    fn instantiate_fsms(
        &mut self,
        builder: &mut ir::Builder,
        coloring: &HashMap<ir::Id, ir::Id>,
        colors_to_max_values: &HashMap<ir::Id, (u64, u64)>,
        colors_to_fsm: &mut HashMap<
            ir::Id,
            (Option<ir::RRC<StaticFSM>>, Option<ir::RRC<StaticFSM>>),
        >,
    ) {
        let color = coloring.get(&self.root.0).expect("couldn't find group");
        match colors_to_fsm.get(color) {
            None => {
                let mut fsm_opt = None;
                let mut repeat_opt = None;
                let (num_states, num_repeats) = colors_to_max_values
                    .get(color)
                    .expect("couldn't find color");
                if *num_states != 1 {
                    // Build parent FSM for the "root" of the tree.
                    let fsm_cell = ir::rrc(StaticFSM::from_basic_info(
                        *num_states,
                        FSMEncoding::Binary, // XXX(Caleb): change this
                        builder,
                    ));
                    fsm_opt = Some(Rc::clone(&fsm_cell));
                    self.fsm_cell = Some(fsm_cell);
                }
                if *num_repeats != 1 {
                    let repeat_counter = ir::rrc(StaticFSM::from_basic_info(
                        *num_repeats,
                        FSMEncoding::Binary, // XXX(Caleb): change this
                        builder,
                    ));
                    repeat_opt = Some(Rc::clone(&repeat_counter));
                    self.iter_count_cell = Some(repeat_counter);
                }
                colors_to_fsm.insert(*color, (fsm_opt, repeat_opt));
            }
            Some((fsm_opt, repeat_opt)) => {
                let fsm_opt_new = match fsm_opt {
                    None => None,
                    Some(x) => Some(Rc::clone(x)),
                };
                let repeat_opt_new = match repeat_opt {
                    None => None,
                    Some(x) => Some(Rc::clone(x)),
                };
                self.fsm_cell = fsm_opt_new;
                self.iter_count_cell = repeat_opt_new;
            }
        }

        for (child, _) in &mut self.children {
            child.instantiate_fsms(
                builder,
                coloring,
                &colors_to_max_values,
                colors_to_fsm,
            );
        }
    }

    fn count_to_n(
        &mut self,
        builder: &mut ir::Builder,
        incr_start_cond: Option<ir::Guard<Nothing>>,
    ) {
        let mut child_index = 0;
        // offload_states are the fsm_states that last >1 cycles
        // i.e., the same as the children.
        let offload_states: BTreeMap<usize, u64> = self
            .delay_map
            .iter()
            .filter_map(|((beg, end), state_type)| match state_type {
                StateType::Delay(_) => None,
                StateType::Offload(offload_state) => {
                    // Only need to include the children that last more than one cycle.
                    let res = if beg + 1 == *end {
                        None
                    } else {
                        Some((child_index, *offload_state))
                    };
                    child_index += 1;
                    res
                }
            })
            .collect();

        // res_vec will contain the assignments that count to n.
        let mut res_vec: Vec<ir::Assignment<Nothing>> = Vec::new();

        // Even if self.num_states == 1, self.latency might be greater than 1,
        // if we're just offloading the computation for the entire time.
        // In this case, we still need the children to count to n.
        if self.latency > 1 {
            for (child, (beg, end)) in self.children.iter_mut() {
                // If beg == 0 and end > 1 then we need to "transfer" the incr_start_condition
                // to the offload state.
                let cond = if *beg == 0 && *end > 1 {
                    incr_start_cond.clone()
                } else {
                    None
                };
                child.count_to_n(builder, cond);
            }
        }

        // Only need an fsm if self.num_states > 1.
        if self.num_states > 1 {
            let parent_fsm = Rc::clone(
                &self
                    .fsm_cell
                    .as_mut()
                    .expect("should have set self.fsm_cell"),
            );
            // Build an adder to increment the parent fsm.
            let (adder_asssigns, adder) =
                parent_fsm.borrow_mut().build_incrementer(builder);
            res_vec.extend(adder_asssigns);
            // Now handle incrementing when we are offloading to a child (we should
            // increment the parent on the final cycle the child is executing).
            for (i, (child, (beg, end))) in self.children.iter_mut().enumerate()
            {
                // Increment parent when child is in final state.
                // fsm.in = fsm == offload_state && child_fsm_in_final_state ? fsm + 1;
                // fsm.write_en = .offload_state && child_fsm_in_final_state ? 1'd1;
                // If the offload state is the last state (end == self.latency) then we don't
                // increment, we need to reset to 0: we will handle that case separately.
                // Also, if end = beg + 1 then the child takes one cycle and we can just
                // unconditionally increment: no need to do all this checking.
                if *end != self.latency && (*beg + 1 != *end) {
                    // We know each offload state corresponds to exactly one child.
                    let child_state = *offload_states.get(&i).expect(
                        "offload states should be a subset of children.",
                    );
                    let in_child_state = parent_fsm
                        .borrow_mut()
                        .query_between(builder, (child_state, child_state + 1));
                    let total_child_latency =
                        child.get_latency() * child.get_num_repeats();
                    let child_final_state = child.query_between(
                        (total_child_latency - 1, total_child_latency),
                        builder,
                    );
                    let parent_fsm_incr =
                        parent_fsm.borrow_mut().conditional_increment(
                            in_child_state.and(child_final_state),
                            Rc::clone(&adder),
                            builder,
                        );
                    res_vec.extend(parent_fsm_incr);
                }
            }

            // If incr_start_cond.is_some(), then we have to specially take into
            // account this scenario when incrementing the FSM.
            let other_incr_condition = match incr_start_cond {
                None => ir::Guard::True,
                Some(g) => {
                    // If we offload the 0->1 transition, then no need to do this.
                    let offload_first_transition =
                        if let Some(((beg, end), state_type)) =
                            self.delay_map.iter().next()
                        {
                            matches!(state_type, StateType::Offload(_))
                                && *beg == 0
                                && *end > 1
                        } else {
                            false
                        };
                    // If we do not offload, then we need to guard this transition.
                    if offload_first_transition {
                        ir::Guard::True
                    } else {
                        let first_state = self.get_fsm_query((0, 1), builder);
                        res_vec.extend(
                            parent_fsm.borrow_mut().conditional_increment(
                                first_state.clone().and(g),
                                Rc::clone(&adder),
                                builder,
                            ),
                        );
                        first_state.not()
                    }
                }
            };

            // Increment when fsm is not in an offload state and not in final fsm state.
            // (and must look at other_incr_condition, in case `incr_start_cond`
            // is something).
            let mut offload_state_guard: ir::Guard<Nothing> =
                ir::Guard::Not(Box::new(ir::Guard::True));
            for offload_state in offload_states.values() {
                // Creating a guard that checks whether the parent fsm is
                // in an offload state.
                offload_state_guard.update(|g| {
                    g.or(*parent_fsm.borrow_mut().query_between(
                        builder,
                        (*offload_state, offload_state + 1),
                    ))
                });
            }
            let not_offload_state = offload_state_guard.not();
            // Getting final state for the parent fsm.
            let final_fsm_state =
                self.get_fsm_query((self.latency - 1, self.latency), builder);
            let not_final_state = final_fsm_state.clone().not();
            res_vec.extend(
                parent_fsm.borrow_mut().conditional_increment(
                    not_offload_state
                        .and(not_final_state.clone().and(other_incr_condition)),
                    Rc::clone(&adder),
                    builder,
                ),
            );
            // reset at final fsm_state.
            res_vec.extend(
                parent_fsm
                    .borrow_mut()
                    .conditional_reset(final_fsm_state.clone(), builder),
            );
        }

        let final_fsm_state = if self.latency == 1 {
            ir::Guard::True
        } else {
            // Getting final state for the parent fsm.
            self.get_fsm_query((self.latency - 1, self.latency), builder)
        };

        // Handle repeats. (Should increment repeat when fsm is in final state.)
        if self.num_repeats != 1 {
            let repeat_fsm = Rc::clone(
                &self
                    .iter_count_cell
                    .as_mut()
                    .expect("should have set self.iter_count_cell"),
            );
            let (repeat_adder_assigns, repeat_adder) =
                repeat_fsm.borrow_mut().build_incrementer(builder);
            let final_repeat_state = *repeat_fsm.borrow_mut().query_between(
                builder,
                (self.num_repeats - 1, self.num_repeats),
            );
            let not_final_repeat_state = final_repeat_state.clone().not();
            res_vec.extend(repeat_adder_assigns);
            // repeat_fsm = fsm_in_final_state && not_final_repeat_state? repeat_fsm + 1
            res_vec.extend(repeat_fsm.borrow_mut().conditional_increment(
                final_fsm_state.clone().and(not_final_repeat_state),
                repeat_adder,
                builder,
            ));
            // repeat_fsm = fsm_in_final_state and final_repeat_state? 0
            res_vec.extend(repeat_fsm.borrow_mut().conditional_reset(
                final_fsm_state.clone().and(final_repeat_state),
                builder,
            ));
        }

        // Extend root assigns to include `res_vec` (which counts to n).
        self.root.1.extend(res_vec);
    }

    fn realize(
        &mut self,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        fsm_info_map: &mut HashMap<
            ir::Id,
            (ir::Id, ir::Guard<Nothing>, ir::Guard<Nothing>),
        >,
        group_rewrites: &mut HashMap<ir::Canonical, ir::RRC<ir::Port>>,
        builder: &mut ir::Builder,
    ) {
        // Get static grouo we are "realizing".
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
        // Realize the static %[i:j] guards to fsm checks.
        // This is significantly more complicated with a tree structure.
        let mut assigns = static_group
            .borrow()
            .assignments
            .clone()
            .into_iter()
            .map(|assign| self.make_assign_dyn(assign, false, false, builder))
            .collect_vec();

        // Add assignment `group[done] = ud.out`` to the new group.
        structure!( builder; let ud = prim undef(1););
        let early_reset_done_assign = build_assignments!(
          builder;
          early_reset_group["done"] = ? ud["out"];
        );
        assigns.extend(early_reset_done_assign);
        // Adding the count_to_n assigns;
        assigns.extend(std::mem::take(&mut self.root.1));
        self.root.1 = assigns.clone();

        early_reset_group.borrow_mut().assignments = assigns;
        early_reset_group.borrow_mut().attributes =
            static_group.borrow().attributes.clone();

        // Now we have to update the fields with a bunch of information.
        // This makes it easier when we have to build wrappers, rewrite ports, etc.

        // Map the static group name -> early reset group name.
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

        let fsm_identifier = match self.fsm_cell.as_ref() {
            None => self.root.0,
            Some(fsm_rc) => fsm_rc.borrow().fsm_cell.borrow().name(),
        };
        let total_latency = self.latency * self.num_repeats;
        fsm_info_map.insert(
            early_reset_group.borrow().name(),
            (
                fsm_identifier,
                self.query_between((0, 1), builder),
                self.query_between((total_latency - 1, total_latency), builder),
            ),
        );

        // Recursively realize each child.
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

    fn transform_static_assigns(
        &mut self,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        group_rewrites: &mut HashMap<ir::Canonical, ir::RRC<ir::Port>>,
        builder: &mut ir::Builder,
    ) {
        // Get static grouo we are "realizing".
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
        // Realize the static %[i:j] guards to fsm checks.
        // This is significantly more complicated with a tree structure.
        let mut assigns = static_group
            .borrow()
            .assignments
            .clone()
            .into_iter()
            .map(|assign| self.make_assign_dyn(assign, false, true, builder))
            .collect_vec();

        // Add assignment `group[done] = ud.out`` to the new group.
        structure!( builder; let ud = prim undef(1););
        let early_reset_done_assign = build_assignments!(
          builder;
          early_reset_group["done"] = ? ud["out"];
        );
        assigns.extend(early_reset_done_assign);
        // Adding the count_to_n assigns;
        assigns.extend(std::mem::take(&mut self.root.1));
        self.root.1 = assigns.clone();

        early_reset_group.borrow_mut().assignments = assigns;
        early_reset_group.borrow_mut().attributes =
            static_group.borrow().attributes.clone();

        // Now we have to update the fields with a bunch of information.
        // This makes it easier when we have to build wrappers, rewrite ports, etc.

        // Map the static group name -> early reset group name.
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

        // Recursively realize each child.
        self.children.iter_mut().for_each(|(child, _)| {
            child.transform_static_assigns(
                static_groups,
                reset_early_map,
                group_rewrites,
                builder,
            )
        })
    }

    // Restructure an (i,j) query into:
    // (beg, middle, end) query.
    // This is best explained by examples: Sup
    // Suppose latency = 5, num repeats = 10.
    // Suppose we query %[3:32].
    // beg = Some(0, 3-5). 0 bc we are on the 0th iteration,
    // and only cycles 3-5 of that iteration.
    // middle = Some([1,6)). These are the iterations for which the query is true
    // throughout the entire iteration.
    // end = Some(6,0-2). 6 bc 6th iteration, 0-2 because only cycles 0-2 of that
    // iteration.
    fn restructure_query(
        &self,
        query: (u64, u64),
    ) -> (
        Option<(u64, (u64, u64))>,
        Option<(u64, u64)>,
        Option<(u64, (u64, u64))>,
    ) {
        // Splitting query into:
        // (beg_iter_query, end_iter_query) is an inclusive (both sides) query
        // on the iterations we are active for.
        // (beg_fsm_query, end_fsm_query) is the fsm query we should be supporting.
        let (beg_query, end_query) = query;
        let (beg_iter_query, beg_fsm_query) =
            (beg_query / self.latency, beg_query % self.latency);
        let (end_iter_query, mut end_fsm_query) =
            ((end_query - 1) / self.latency, (end_query) % self.latency);
        if end_fsm_query == 0 {
            end_fsm_query = self.latency;
        }
        let x = if beg_iter_query == end_iter_query {
            let repeat_query = beg_iter_query;
            let fsm_query = (beg_fsm_query, end_fsm_query);
            let res = Some((repeat_query, fsm_query));
            (res, None, None)
        } else if beg_iter_query + 1 == end_iter_query {
            let middle_res = None;

            let repeat_query0 = beg_iter_query;
            let fsm_query0 = (beg_fsm_query, self.latency);
            let beg_res = Some((repeat_query0, fsm_query0));
            // if beg_fsm_query == 0 {
            //     beg_res = None;
            //     middle_res = Some((beg_iter_query, beg_iter_query + 1));
            // }

            let repeat_query1 = end_iter_query;
            let fsm_query1 = (0, end_fsm_query);
            let end_res = Some((repeat_query1, fsm_query1));
            // if end_fsm_query == self.latency {
            //     end_res = None;
            //     middle_res = match middle_res {
            //         Some((i, _)) => Some((i, end_iter_query + 1)),
            //         None => Some((end_iter_query, end_iter_query + 1)),
            //     }
            // }

            (beg_res, middle_res, end_res)
        } else {
            let mut unconditional_repeat_query =
                (beg_iter_query + 1, end_iter_query);

            let repeat_query0 = beg_iter_query;
            let fsm_query0 = (beg_fsm_query, self.latency);
            let mut beg_res = Some((repeat_query0, fsm_query0));
            if beg_fsm_query == 0 {
                beg_res = None;
                unconditional_repeat_query.0 -= 1;
            }

            let repeat_query1 = end_iter_query;
            let fsm_query1 = (0, end_fsm_query);
            let mut end_res = Some((repeat_query1, fsm_query1));
            if end_fsm_query == self.latency {
                end_res = None;
                unconditional_repeat_query.1 += 1;
            }

            (beg_res, Some(unconditional_repeat_query), end_res)
        };
        x
    }

    fn offload_entire_latency(&self) -> bool {
        self.children.len() == 1
            && self
                .children
                .iter()
                .any(|(_, (beg, end))| *beg == 0 && *end == self.latency)
            && self.num_states == 1
    }

    // Given query (i,j), get the fsm query for cycles (i,j).
    // Complicated by the offloading to children.
    // We use a similar resturcturing of the query to (beg, middle, end).
    fn get_fsm_query(
        &mut self,
        query: (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        if self.latency == 1 {
            return ir::Guard::True;
        }
        let fsm_cell_opt = self.fsm_cell.as_ref();
        if fsm_cell_opt.is_none() {
            assert!(self.offload_entire_latency());
            let (only_child, _) = self.children.iter_mut().next().unwrap();
            return only_child.query_between(query, builder);
        }
        let fsm_cell: Rc<std::cell::RefCell<StaticFSM>> =
            Rc::clone(fsm_cell_opt.expect("just checked if None"));

        let (query_beg, query_end) = query;
        let mut beg_interval = ir::Guard::True.not();
        let mut end_interval = ir::Guard::True.not();
        let mut middle_interval = None;
        let mut child_index = 0;
        for ((beg, end), state_type) in self.delay_map.iter() {
            // Check if the query encompasses the entire interval.
            // If so, we add it to the "middle" interval.
            if query_beg <= *beg && *end <= query_end {
                let interval_to_add = match state_type {
                    StateType::Delay(delay) => (beg - delay, end - delay),
                    StateType::Offload(offload_state) => {
                        (*offload_state, offload_state + 1)
                    }
                };
                match middle_interval {
                    None => middle_interval = Some(interval_to_add),
                    Some((cur_start, cur_end)) => {
                        assert!(cur_end == interval_to_add.0);
                        middle_interval = Some((cur_start, interval_to_add.1));
                    }
                }
            }
            // Otherwise check if the beginning of the query lies within the
            // interval. This should only happen once.
            else if *beg <= query_beg && query_beg < *end {
                assert!(beg_interval.is_false());
                match state_type {
                    StateType::Delay(delay) => {
                        let translated_query = (
                            query_beg - delay,
                            // This query either stretches into another interval, or
                            // ends within the interavl: we want to capture both of these choices.
                            std::cmp::min(query_end - delay, end - delay),
                        );
                        beg_interval = *fsm_cell
                            .borrow_mut()
                            .query_between(builder, translated_query);
                    }
                    StateType::Offload(offload_state) => {
                        let in_offload_state =
                            *fsm_cell.borrow_mut().query_between(
                                builder,
                                (*offload_state, offload_state + 1),
                            );
                        let (child, _) =
                            self.children.get_mut(child_index).unwrap();
                        let child_query = child.query_between(
                            (
                                query_beg - beg,
                                std::cmp::min(
                                    query_end - beg,
                                    child.get_latency()
                                        * child.get_num_repeats(),
                                ),
                            ),
                            builder,
                        );
                        beg_interval = in_offload_state.and(child_query);
                    }
                };
            } else if *beg < query_end && query_end <= *end {
                assert!(end_interval.is_false());
                match state_type {
                    StateType::Delay(delay) => {
                        end_interval = *fsm_cell.borrow_mut().query_between(
                            builder,
                            (
                                std::cmp::max(query_beg, *beg) - delay,
                                query_end - delay,
                            ),
                        );
                    }
                    StateType::Offload(offload_state) => {
                        let in_offload_state =
                            *fsm_cell.borrow_mut().query_between(
                                builder,
                                (*offload_state, offload_state + 1),
                            );
                        let (child, _) =
                            self.children.get_mut(child_index).unwrap();
                        // XXX(Caleb) I think we can do this. We know this must
                        // stretch into a previous interval: otherwise, it
                        // would have been caught by the previous elif condition.
                        let child_query = child
                            .query_between((0, (query_end - beg)), builder);
                        beg_interval = in_offload_state.and(child_query);
                    }
                };
            }
            if matches!(state_type, StateType::Offload(_)) {
                child_index += 1;
            }
        }

        let middle_query = match middle_interval {
            None => Box::new(ir::Guard::True.not()),
            Some((i, j)) => self
                .fsm_cell
                .as_mut()
                .unwrap()
                .borrow_mut()
                .query_between(builder, (i, j)),
        };

        beg_interval.or(end_interval.or(*middle_query))
    }

    fn get_repeat_query(
        &mut self,
        query: (u64, u64),
        builder: &mut ir::Builder,
    ) -> Box<ir::Guard<Nothing>> {
        match self.num_repeats {
            1 => {
                assert!(query.0 == 0 && query.1 == 1);
                Box::new(ir::Guard::True)
            }
            _ => self
                .iter_count_cell
                .as_mut()
                .expect("querying repeat implies cell exists")
                .borrow_mut()
                .query_between(builder, (query.0, query.1)),
        }
    }

    fn check_iter_fsm_state(
        &mut self,
        (repeat_query, fsm_query): (u64, (u64, u64)),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        let fsm_guard = if 0 == fsm_query.0 && self.latency == fsm_query.1 {
            ir::Guard::True
        } else {
            self.get_fsm_query(fsm_query, builder)
        };

        let counter_guard =
            self.get_repeat_query((repeat_query, repeat_query + 1), builder);
        ir::Guard::And(Box::new(fsm_guard), counter_guard)
    }

    fn query_between(
        &mut self,
        query: (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        let (query0, repeat_query, query1) = self.restructure_query(query);
        let g0 = match query0 {
            None => ir::Guard::True.not(),
            Some(q0) => self.check_iter_fsm_state(q0, builder),
        };
        let g1 = match query1 {
            None => ir::Guard::True.not(),
            Some(q1) => self.check_iter_fsm_state(q1, builder),
        };

        let rep_query = match repeat_query {
            None => Box::new(ir::Guard::True.not()),
            Some(rq) => self.get_repeat_query(rq, builder),
        };
        g0.or(g1.or(*rep_query))
    }

    // Takes in a static guard `guard`, and returns equivalent dynamic guard
    // The only thing that actually changes is the Guard::Info case
    // We need to turn static_timing to dynamic guards using `fsm`.
    fn make_guard_dyn(
        &mut self,
        guard: ir::Guard<ir::StaticTiming>,
        global_view: bool,
        ignore_timing: bool,
        builder: &mut ir::Builder,
    ) -> Box<ir::Guard<Nothing>> {
        match guard {
            ir::Guard::Or(l, r) => Box::new(ir::Guard::Or(
                self.make_guard_dyn(*l, global_view, ignore_timing, builder),
                self.make_guard_dyn(*r, global_view, ignore_timing, builder),
            )),
            ir::Guard::And(l, r) => Box::new(ir::Guard::And(
                self.make_guard_dyn(*l, global_view, ignore_timing, builder),
                self.make_guard_dyn(*r, global_view, ignore_timing, builder),
            )),
            ir::Guard::Not(g) => Box::new(ir::Guard::Not(self.make_guard_dyn(
                *g,
                global_view,
                ignore_timing,
                builder,
            ))),
            ir::Guard::CompOp(op, l, r) => {
                Box::new(ir::Guard::CompOp(op, l, r))
            }
            ir::Guard::Port(p) => Box::new(ir::Guard::Port(p)),
            ir::Guard::True => Box::new(ir::Guard::True),
            ir::Guard::Info(static_timing) => {
                if ignore_timing {
                    assert!(static_timing.get_interval() == (0, 1));
                    return Box::new(ir::Guard::True);
                }
                if global_view {
                    Box::new(
                        self.query_between(
                            static_timing.get_interval(),
                            builder,
                        ),
                    )
                } else {
                    Box::new(
                        self.get_fsm_query(
                            static_timing.get_interval(),
                            builder,
                        ),
                    )
                }
            }
        }
    }

    // Takes in static assignment `assign` and returns a dynamic assignments
    // Mainly transforms the guards from %[2:3] -> fsm.out >= 2 & fsm.out <= 3
    pub fn make_assign_dyn(
        &mut self,
        assign: ir::Assignment<ir::StaticTiming>,
        global_view: bool,
        ignore_timing: bool,
        builder: &mut ir::Builder,
    ) -> ir::Assignment<Nothing> {
        ir::Assignment {
            src: assign.src,
            dst: assign.dst,
            attributes: assign.attributes,
            guard: self.make_guard_dyn(
                *assign.guard,
                global_view,
                ignore_timing,
                builder,
            ),
        }
    }

    pub fn get_all_nodes(&self) -> Vec<ir::Id> {
        let mut res = vec![self.root.0];
        for (child, _) in &self.children {
            res.extend(child.get_all_nodes())
        }
        return res;
    }

    pub fn add_conflicts(&self, conflict_graph: &mut GraphColoring<ir::Id>) {
        let root_name = self.root.0;
        for (child, _) in &self.children {
            for sgroup in &child.get_all_nodes() {
                conflict_graph.insert_conflict(&root_name, sgroup);
            }
            child.add_conflicts(conflict_graph);
        }
    }

    pub fn get_max_value<F>(&self, name: &ir::Id, f: &F) -> u64
    where
        F: Fn(&Tree) -> u64,
    {
        let mut cur_max = 1;
        if self.root.0 == name {
            cur_max = std::cmp::max(cur_max, f(self));
        }
        for (child, _) in &self.children {
            cur_max = std::cmp::max(cur_max, child.get_max_value(name, f));
        }
        cur_max
    }
}

pub struct ParTree {
    pub group_name: ir::Id,
    pub latency: u64,
    pub threads: Vec<(FSMTree, (u64, u64))>,
    pub num_repeats: u64,
}

impl ParTree {
    pub fn instantiate_fsms(
        &mut self,
        builder: &mut ir::Builder,
        coloring: &HashMap<ir::Id, ir::Id>,
        colors_to_max_values: &HashMap<ir::Id, (u64, u64)>,
        colors_to_fsm: &mut HashMap<
            ir::Id,
            (Option<ir::RRC<StaticFSM>>, Option<ir::RRC<StaticFSM>>),
        >,
    ) {
        for (child, _) in &mut self.threads {
            child.instantiate_fsms(
                builder,
                coloring,
                &colors_to_max_values,
                colors_to_fsm,
            );
        }
    }

    pub fn count_to_n(
        &mut self,
        builder: &mut ir::Builder,
        incr_start_cond: Option<ir::Guard<Nothing>>,
    ) {
        for (child, _) in &mut self.threads {
            child.count_to_n(builder, incr_start_cond.clone());
        }
    }
    pub fn get_longest_tree(&mut self) -> &mut Tree {
        let max = self.threads.iter_mut().max_by_key(|(child, _)| {
            (child.get_latency() * child.get_num_repeats()) as i64
        });
        if let Some((max_child, _)) = max {
            match max_child {
                FSMTree::Par(par_tree) => par_tree.get_longest_tree(),
                FSMTree::Tree(tree) => tree,
            }
        } else {
            panic!("Field is empty or no maximum value found");
        }
    }

    pub fn transform_static_assigns(
        &mut self,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        group_rewrites: &mut HashMap<ir::Canonical, ir::RRC<ir::Port>>,
        builder: &mut ir::Builder,
    ) {
        // Get static grouo we are "realizing".
        let static_group = Rc::clone(
            &static_groups
                .iter()
                .find(|sgroup| sgroup.borrow().name() == self.group_name)
                .unwrap(),
        );
        // Create the dynamic "early reset group" that will replace the static group.
        let static_group_name = static_group.borrow().name();
        let mut early_reset_name = static_group_name.to_string();
        early_reset_name.insert_str(0, "early_reset_");
        let early_reset_group = builder.add_group(early_reset_name);

        let longest_tree = self.get_longest_tree();
        // Use the longest tree to dictate the assignments of the others.
        let mut assigns = static_group
            .borrow()
            .assignments
            .clone()
            .into_iter()
            .map(|assign| {
                longest_tree.make_assign_dyn(assign, true, true, builder)
            })
            .collect_vec();

        // Add assignment `group[done] = ud.out`` to the new group.
        structure!( builder; let ud = prim undef(1););
        let early_reset_done_assign = build_assignments!(
          builder;
          early_reset_group["done"] = ? ud["out"];
        );
        assigns.extend(early_reset_done_assign);

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

        self.threads.iter_mut().for_each(|(child, _)| {
            child.transform_static_assigns(
                static_groups,
                reset_early_map,
                group_rewrites,
                builder,
            )
        })
    }

    pub fn realize(
        &mut self,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        fsm_info_map: &mut HashMap<
            ir::Id,
            (ir::Id, ir::Guard<Nothing>, ir::Guard<Nothing>),
        >,
        group_rewrites: &mut HashMap<ir::Canonical, ir::RRC<ir::Port>>,
        builder: &mut ir::Builder,
    ) {
        // Get static grouo we are "realizing".
        let static_group = Rc::clone(
            &static_groups
                .iter()
                .find(|sgroup| sgroup.borrow().name() == self.group_name)
                .unwrap(),
        );
        // Create the dynamic "early reset group" that will replace the static group.
        let static_group_name = static_group.borrow().name();
        let mut early_reset_name = static_group_name.to_string();
        early_reset_name.insert_str(0, "early_reset_");
        let early_reset_group = builder.add_group(early_reset_name);

        let longest_tree = self.get_longest_tree();

        // Use the longest tree to dictate the assignments of the others.
        let mut assigns = static_group
            .borrow()
            .assignments
            .clone()
            .into_iter()
            .map(|assign| {
                longest_tree.make_assign_dyn(assign, true, false, builder)
            })
            .collect_vec();

        // Add assignment `group[done] = ud.out`` to the new group.
        structure!( builder; let ud = prim undef(1););
        let early_reset_done_assign = build_assignments!(
          builder;
          early_reset_group["done"] = ? ud["out"];
        );
        assigns.extend(early_reset_done_assign);

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

        let total_latency = self.latency * self.num_repeats;
        fsm_info_map.insert(
            early_reset_group.borrow().name(),
            (
                self.group_name,
                self.query_between((0, 1), builder),
                self.query_between((total_latency - 1, total_latency), builder),
            ),
        );

        // Recursively realize each child.
        self.threads.iter_mut().for_each(|(child, _)| {
            child.realize(
                static_groups,
                reset_early_map,
                fsm_info_map,
                group_rewrites,
                builder,
            )
        })
    }

    pub fn query_between(
        &mut self,
        query: (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        let longest_tree = self.get_longest_tree();
        longest_tree.query_between(query, builder)
    }

    pub fn get_all_nodes(&self) -> Vec<ir::Id> {
        let mut res = vec![];
        for (thread, _) in &self.threads {
            res.extend(thread.get_all_nodes())
        }
        return res;
    }

    pub fn add_conflicts(&self, conflict_graph: &mut GraphColoring<ir::Id>) {
        for ((thread1, _), (thread2, _)) in
            self.threads.iter().tuple_combinations()
        {
            for sgroup1 in thread1.get_all_nodes() {
                for sgroup2 in thread2.get_all_nodes() {
                    conflict_graph.insert_conflict(&sgroup1, &sgroup2);
                }
            }
            thread1.add_conflicts(conflict_graph);
            thread2.add_conflicts(conflict_graph);
        }
    }

    pub fn get_max_value<F>(&self, name: &ir::Id, f: &F) -> u64
    where
        F: Fn(&Tree) -> u64,
    {
        let mut cur_max = 1;
        for (thread, _) in &self.threads {
            cur_max = std::cmp::max(cur_max, thread.get_max_value(name, f));
        }
        cur_max
    }
}

impl FSMTree {
    // Looks recursively thru guard to transform %[0:n] into %0 | %[1:n].
    fn preprocess_static_interface_guard(
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
                let left = Self::preprocess_static_interface_guard(
                    *l,
                    Rc::clone(&comp_sig),
                );
                let right =
                    Self::preprocess_static_interface_guard(*r, comp_sig);
                ir::Guard::and(left, right)
            }
            ir::Guard::Or(l, r) => {
                let left = Self::preprocess_static_interface_guard(
                    *l,
                    Rc::clone(&comp_sig),
                );
                let right =
                    Self::preprocess_static_interface_guard(*r, comp_sig);
                ir::Guard::or(left, right)
            }
            ir::Guard::Not(g) => {
                let a = Self::preprocess_static_interface_guard(*g, comp_sig);
                ir::Guard::Not(Box::new(a))
            }
            _ => guard,
        }
    }

    // Looks recursively thru assignment's guard to %[0:n] into %0 | %[1:n].
    pub fn preprocess_static_interface_assigns(
        assign: &mut ir::Assignment<ir::StaticTiming>,
        comp_sig: ir::RRC<ir::Cell>,
    ) {
        assign
            .guard
            .update(|g| Self::preprocess_static_interface_guard(g, comp_sig));
    }
}
