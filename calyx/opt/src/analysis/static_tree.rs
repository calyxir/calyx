use super::{FSMEncoding, StaticFSM};
use calyx_ir::{self as ir};
use calyx_ir::{Nothing, build_assignments};
use calyx_ir::{guard, structure};
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};
use std::ops::Not;
use std::rc::Rc;

use super::GraphColoring;

/// Optional Rc of a RefCell of a StaticFSM object.
type OptionalStaticFSM = Option<ir::RRC<StaticFSM>>;
/// Query (i ,(j,k)) that corresponds to:
/// Am I in iteration i, and between cylces j and k within
/// that query?
type SingleIterQuery = (u64, (u64, u64));
/// Query (i,j) that corresponds to:
/// Am I between iterations i and j, inclusive?
type ItersQuery = (u64, u64);

/// Helpful for translating queries for the FSMTree structure.
/// Because of the tree structure, %[i:j] is no longer is always equal to i <= fsm < j.
/// Offload(i) means the FSM is offloading when fsm == i: so if the fsm == i,
/// we need to look at the children to know what cycle we are in exactly.
/// Normal(i,j) means the FSM is outputing (i..j), incrementing each cycle (i.e.,
/// like normal) and not offloading. Note that even though the FSM is outputting
/// i..j each cycle, that does not necesarily mean we are in cycles i..j (due
/// to offloading performed in the past.)
#[derive(Debug)]
pub enum StateType {
    Normal((u64, u64)),
    Offload(u64),
}

/// Node can either be a SingleNode (i.e., a single node) or ParNodes (i.e., a group of
/// nodes that are executing in parallel).
/// Most methods in `Node` simply call the equivalent methods for each
/// of the two possible variants.
/// Perhaps could be more compactly implemented as a Trait.
pub enum Node {
    Single(SingleNode),
    Par(ParNodes),
}

// The following methods are used to actually instantiate the FSMTree structure
// and compile static groups/control to dynamic groups/control.
impl Node {
    /// Instantiate the necessary registers.
    /// The equivalent methods for the two variants contain more implementation
    /// details.
    /// `coloring`, `colors_to_max_values`, and `colors_to_fsm` are necessary
    /// to know whether we actually need to instantiate a new FSM, or we can
    /// juse use another node's FSM.
    pub fn instantiate_fsms(
        &mut self,
        builder: &mut ir::Builder,
        coloring: &HashMap<ir::Id, ir::Id>,
        colors_to_max_values: &HashMap<ir::Id, (u64, u64)>,
        colors_to_fsm: &mut HashMap<
            ir::Id,
            (OptionalStaticFSM, OptionalStaticFSM),
        >,
        one_hot_cutoff: u64,
    ) {
        match self {
            Node::Single(single_node) => single_node.instantiate_fsms(
                builder,
                coloring,
                colors_to_max_values,
                colors_to_fsm,
                one_hot_cutoff,
            ),
            Node::Par(par_nodes) => par_nodes.instantiate_fsms(
                builder,
                coloring,
                colors_to_max_values,
                colors_to_fsm,
                one_hot_cutoff,
            ),
        }
    }

    /// Count to n. Need to call `instantiate_fsms` before calling `count_to_n`.
    /// The equivalent methods for the two variants contain more implementation
    /// details.
    /// `incr_start_cond` can optionally guard the 0->1 transition.
    pub fn count_to_n(
        &mut self,
        builder: &mut ir::Builder,
        incr_start_cond: Option<ir::Guard<Nothing>>,
    ) {
        match self {
            Node::Single(single_node) => {
                single_node.count_to_n(builder, incr_start_cond)
            }
            Node::Par(par_nodes) => {
                par_nodes.count_to_n(builder, incr_start_cond)
            }
        }
    }

    /// "Realize" the static groups into dynamic groups.
    /// The main challenge is converting %[i:j] into fsm guards.
    /// Need to call `instantiate_fsms` and
    /// `count_to_n` before calling `realize`.
    /// The equivalent methods for the two variants contain more implementation
    /// details.
    /// `reset_early_map`, `fsm_info_map`, and `group_rewrites` are just metadata
    /// to make it easier to rewrite control, add wrappers, etc.
    pub fn realize(
        &mut self,
        ignore_timing_guards: bool,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        fsm_info_map: &mut HashMap<
            ir::Id,
            (ir::Id, ir::Guard<Nothing>, ir::Guard<Nothing>),
        >,
        group_rewrites: &mut ir::rewriter::PortRewriteMap,
        builder: &mut ir::Builder,
    ) {
        match self {
            Node::Single(single_node) => single_node.realize(
                ignore_timing_guards,
                static_groups,
                reset_early_map,
                fsm_info_map,
                group_rewrites,
                builder,
            ),
            Node::Par(par_nodes) => par_nodes.realize(
                ignore_timing_guards,
                static_groups,
                reset_early_map,
                fsm_info_map,
                group_rewrites,
                builder,
            ),
        }
    }

    /// Get the equivalent fsm guard when the tree is between cycles i and j, i.e.,
    /// when i <= cycle_count < j.
    /// The equivalent methods for the two variants contain more implementation
    /// details.
    pub fn query_between(
        &mut self,
        query: (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        match self {
            Node::Single(single_node) => {
                single_node.query_between(query, builder)
            }
            Node::Par(par_nodes) => par_nodes.query_between(query, builder),
        }
    }
}

/// The following methods are used to help build the conflict graph for coloring
/// to share FSMs
impl Node {
    /// Get the names of all nodes (i.e., the names of the groups for each node
    /// in the tree).
    pub fn get_all_nodes(&self) -> Vec<ir::Id> {
        match self {
            Node::Single(single_node) => single_node.get_all_nodes(),
            Node::Par(par_nodes) => par_nodes.get_all_nodes(),
        }
    }

    /// Adds conflicts between nodes in the tree that execute at the same time.
    pub fn add_conflicts(&self, conflict_graph: &mut GraphColoring<ir::Id>) {
        match self {
            Node::Single(single_node) => {
                single_node.add_conflicts(conflict_graph)
            }
            Node::Par(par_nodes) => par_nodes.add_conflicts(conflict_graph),
        }
    }

    /// Get max value of all nodes in the tree, according to some function f.
    /// `f` takes in a Tree (i.e., a node type) and returns a `u64`.
    pub fn get_max_value<F>(&self, name: &ir::Id, f: &F) -> u64
    where
        F: Fn(&SingleNode) -> u64,
    {
        match self {
            Node::Single(single_node) => single_node.get_max_value(name, f),
            Node::Par(par_nodes) => par_nodes.get_max_value(name, f),
        }
    }
}

// Used to compile static component interface. This is really annoying to do, since
// for static components, they only need to be guarded for %0, while for static
// groups, they need to be guarded for %[0:n]. This creates some annoying `if`
// statements.
impl Node {
    // Helper to `preprocess_static_interface_assigns`
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

// The following are just standard `getter` methods.
impl Node {
    /// Take the assignments of the root of the tree and return them.
    /// This only works on a single node (i.e., the `Tree`` variant).
    pub fn take_root_assigns(&mut self) -> Vec<ir::Assignment<Nothing>> {
        match self {
            Node::Single(single_node) => {
                std::mem::take(&mut single_node.root.1)
            }
            Node::Par(_) => {
                unreachable!(
                    "Cannot take root assignments of Node::Par variant"
                )
            }
        }
    }

    /// Get the name of the root of the tree and return them.
    /// This only works on a single node (i.e., the `Tree`` variant).
    pub fn get_root_name(&mut self) -> ir::Id {
        match self {
            Node::Single(single_node) => single_node.root.0,
            Node::Par(_) => {
                unreachable!("Cannot take root name of Node::Par variant")
            }
        }
    }

    /// Get the name of the group at the root of the tree (if a `Tree` variant) or
    /// of the equivalent `par` group (i.e., the name of the group that triggers
    /// execution of all the trees) if a `Par` variant.
    pub fn get_group_name(&self) -> ir::Id {
        match self {
            Node::Single(single_node) => single_node.root.0,
            Node::Par(par_nodes) => par_nodes.group_name,
        }
    }

    /// Gets latency of the overall tree.
    pub fn get_latency(&self) -> u64 {
        match self {
            Node::Single(single_node) => single_node.latency,
            Node::Par(par_nodes) => par_nodes.latency,
        }
    }

    /// Gets the children of root of the tree (if a `Tree` variant) or
    /// of the threads (i.e., trees) that are scheduled to execute (if a `Par`
    /// variant.)
    pub fn get_children(&mut self) -> &mut Vec<(Node, (u64, u64))> {
        match self {
            Node::Single(single_node) => &mut single_node.children,
            Node::Par(par_nodes) => &mut par_nodes.threads,
        }
    }

    /// Get number of repeats.
    fn get_num_repeats(&self) -> u64 {
        match self {
            Node::Single(single_node) => single_node.num_repeats,
            Node::Par(par_nodes) => par_nodes.num_repeats,
        }
    }
}

/// `SingleNode` struct.
pub struct SingleNode {
    /// latency of one iteration.
    pub latency: u64,
    /// number of repeats. (So "total" latency = `latency` x `num_repeats`)
    pub num_repeats: u64,
    /// number of states in this node
    pub num_states: u64,
    /// (name of static group, assignments to build a corresponding dynamic group)
    pub root: (ir::Id, Vec<ir::Assignment<Nothing>>),
    ///  maps cycles (i,j) -> fsm state type.
    ///  Here is an example FSM schedule:
    ///                           Cycles     FSM State (i.e., `fsm.out`)
    ///                           (0..10) ->  Normal[0,10)
    ///                           (10..30) -> Offload(10) // Offloading to child
    ///                           (30..40) -> Normal[11, 21)
    ///                           (40,80) ->  Offload(21)
    ///                           (80,100)->  Normal[22, 42)
    pub fsm_schedule: BTreeMap<(u64, u64), StateType>,
    /// vec of (Node Object, cycles for which that child is executing).
    /// Note that you can build `fsm_schedule` from just this information,
    /// but it's convenient to have `fsm_schedule` avaialable.
    pub children: Vec<(Node, (u64, u64))>,
    /// Keep track of where we are within a single iteration.
    /// If `latency` == 1, then we don't need an `fsm_cell`.
    pub fsm_cell: Option<ir::RRC<StaticFSM>>,
    /// Keep track of which iteration we are on. If iteration count == 1, then
    /// we don't need an `iter_count_cell`.
    pub iter_count_cell: Option<ir::RRC<StaticFSM>>,
}

impl SingleNode {
    /// Instantiates the necessary registers.
    /// Because we share FSM registers, it's possible that this register has already
    /// been instantiated.
    /// Therefore we take in a bunch of data structures to keep track of coloring:
    ///   - `coloring` that maps group names -> colors,
    ///   - `colors_to_max_values` which maps colors -> (max latency, max_num_repeats)
    ///     (we need to make sure that when we instantiate a color,
    ///     we give enough bits to support the maximum latency/num_repeats that will be
    ///     used for that color)
    ///   - `colors_to_fsm`
    ///     which maps colors to (fsm_register, iter_count_register): fsm_register counts
    ///     up for a single iteration, iter_count_register counts the number of iterations
    ///     that have passed.
    ///
    /// Note that it is not always necessary to instantiate one or both registers (e.g.,
    /// if num_repeats == 1 then you don't need an iter_count_register).
    ///
    /// `one_hot_cutoff` is the cutoff to choose between binary and one hot encoding.
    /// Any number of states greater than the cutoff will get binary encoding.
    fn instantiate_fsms(
        &mut self,
        builder: &mut ir::Builder,
        coloring: &HashMap<ir::Id, ir::Id>,
        colors_to_max_values: &HashMap<ir::Id, (u64, u64)>,
        colors_to_fsm: &mut HashMap<
            ir::Id,
            (OptionalStaticFSM, OptionalStaticFSM),
        >,
        one_hot_cutoff: u64,
    ) {
        // Get color assigned to this node.
        let color = coloring.get(&self.root.0).expect("couldn't find group");
        // Check if we've already instantiated the registers for this color.
        match colors_to_fsm.get(color) {
            // We need to create the registers for the colors.
            None => {
                // First we get the maximum num_states and num_repeats
                // for this color so we know how many bits each register needs.
                let (num_states, num_repeats) = colors_to_max_values
                    .get(color)
                    .expect("Couldn't find color");
                // Only need a `self.fsm_cell` if num_states > 1.
                if *num_states != 1 {
                    // Choose encoding based on one_hot_cutoff.
                    let encoding = if *num_states > one_hot_cutoff {
                        FSMEncoding::Binary
                    } else {
                        FSMEncoding::OneHot
                    };
                    let fsm_cell = ir::rrc(StaticFSM::from_basic_info(
                        *num_states,
                        encoding,
                        builder,
                    ));
                    self.fsm_cell = Some(fsm_cell);
                }
                // Only need a `self.iter_count_cell` if num_states > 1.
                if *num_repeats != 1 {
                    let encoding = if *num_repeats > one_hot_cutoff {
                        FSMEncoding::Binary
                    } else {
                        FSMEncoding::OneHot
                    };
                    let repeat_counter = ir::rrc(StaticFSM::from_basic_info(
                        *num_repeats,
                        encoding,
                        builder,
                    ));
                    self.iter_count_cell = Some(repeat_counter);
                }

                // Insert into `colors_to_fsms` so the next time we call this method
                // we see we've already instantiated the registers.
                colors_to_fsm.insert(
                    *color,
                    (
                        self.fsm_cell.as_ref().map(Rc::clone),
                        self.iter_count_cell.as_ref().map(Rc::clone),
                    ),
                );
            }
            Some((fsm_option, repeat_option)) => {
                // Trivially assign to `self.fsm_cell` and `self.iter_count_cell` since
                // we've already created it.
                self.fsm_cell = fsm_option.as_ref().map(Rc::clone);
                self.iter_count_cell = repeat_option.as_ref().map(Rc::clone);
            }
        }

        // Recursively instantiate fsms for all the children.
        for (child, _) in &mut self.children {
            child.instantiate_fsms(
                builder,
                coloring,
                colors_to_max_values,
                colors_to_fsm,
                one_hot_cutoff,
            );
        }
    }

    /// Counts to n.
    /// If `incr_start_cond.is_some()`, then we will add it as an extra
    /// guard guarding the 0->1 transition.
    fn count_to_n(
        &mut self,
        builder: &mut ir::Builder,
        incr_start_cond: Option<ir::Guard<Nothing>>,
    ) {
        // res_vec will contain the assignments that count to n.
        let mut res_vec: Vec<ir::Assignment<Nothing>> = Vec::new();

        // Only need to count up to n if self.num_states > 1.
        // If self.num_states == 1, then either a) latency is 1 cycle or b)
        // we're just offloading the entire time (so the child will count).
        // Either way, there's no need to instantiate a self.fsm_cell to count.
        if self.num_states > 1 {
            // `offload_states` are the fsm_states that last >1 cycles (i.e., states
            // where children are executing, unless the child only lasts one cycle---
            // then we can discount it as an "offload" state).
            let offload_states: Vec<u64> = self
                .fsm_schedule
                .iter()
                .filter_map(|((beg, end), state_type)| match state_type {
                    StateType::Normal(_) => None,
                    StateType::Offload(offload_state) => {
                        // Only need to include the children that last more than one cycle.
                        if beg + 1 == *end {
                            None
                        } else {
                            Some(*offload_state)
                        }
                    }
                })
                .collect();

            // There are two conditions under which we increment the FSM.
            // 1) Increment when we are NOT in an offload state
            // 2) Increment when we ARE in an offload state, but the child being offloaded
            // is in its final state. (intuitively, we need to increment because
            // we know the control is being passed back to parent in the next cycle).
            // (when we are in the final state, we obviously should not increment:
            // we should reset back to 0.)

            let parent_fsm = Rc::clone(
                self.fsm_cell
                    .as_mut()
                    .expect("should have set self.fsm_cell"),
            );

            // Build an adder to increment the parent fsm.
            let (adder_asssigns, adder) =
                parent_fsm.borrow_mut().build_incrementer(builder);
            res_vec.extend(adder_asssigns);

            // Handle situation 1). Increment when we are NOT in an offload state
            res_vec.extend(self.increment_if_not_offloading(
                incr_start_cond.clone(),
                &offload_states,
                Rc::clone(&adder),
                Rc::clone(&parent_fsm),
                builder,
            ));

            // Handle situation 2): Increment when we ARE in an offload state
            // but the child being offloaded is in its final state.
            res_vec.extend(self.increment_if_child_final_state(
                &offload_states,
                adder,
                Rc::clone(&parent_fsm),
                builder,
            ));

            // Reset the FSM when it is at its final fsm_state.
            let final_fsm_state =
                self.get_fsm_query((self.latency - 1, self.latency), builder);
            res_vec.extend(
                parent_fsm
                    .borrow_mut()
                    .conditional_reset(final_fsm_state, builder),
            );
        }

        // If self.num_states > 1, then it's guaranteed that self.latency > 1.
        // However, even if self.num_states == 1, self.latency might still be
        // greater than 1 if we're just offloading the computation for the entire time.
        // In this case, we still need the children to count to n.
        if self.latency > 1 {
            for (child, (beg, end)) in self.children.iter_mut() {
                // If beg == 0 and end > 1 then we need to "transfer" the incr_start_condition
                // to the child so it guards the 0->1 transition.
                let cond = if *beg == 0 && *end > 1 {
                    incr_start_cond.clone()
                } else {
                    None
                };
                // Recursively call `count_to_n`
                child.count_to_n(builder, cond);
            }
        }

        // Handle repeats (i.e., make sure we actually interate `self.num_repeats` times).
        if self.num_repeats != 1 {
            // If self.latency == 10, then we should increment the self.iter_count_cell
            // each time fsm == 9, i.e., `final_fsm_state`.
            let final_fsm_state =
                self.get_fsm_query((self.latency - 1, self.latency), builder);

            // `repeat_fsm` store number of iterations.
            let repeat_fsm = Rc::clone(
                self.iter_count_cell
                    .as_mut()
                    .expect("should have set self.iter_count_cell"),
            );
            // Build an incrementer to increment `self.iter_count_cell`.
            let (repeat_adder_assigns, repeat_adder) =
                repeat_fsm.borrow_mut().build_incrementer(builder);
            // We shouldn't increment `self.iter_count_cell` if we are in the final iteration:
            // we should reset it instead.
            let final_repeat_state = *repeat_fsm.borrow_mut().query_between(
                builder,
                (self.num_repeats - 1, self.num_repeats),
            );
            let not_final_repeat_state = final_repeat_state.clone().not();
            res_vec.extend(repeat_adder_assigns);
            // Incrementing self.iter_count_cell when appropriate.
            res_vec.extend(repeat_fsm.borrow_mut().conditional_increment(
                final_fsm_state.clone().and(not_final_repeat_state),
                repeat_adder,
                builder,
            ));
            // Resetting self.iter_count_cell when appropriate.
            res_vec.extend(repeat_fsm.borrow_mut().conditional_reset(
                final_fsm_state.clone().and(final_repeat_state),
                builder,
            ));
        }

        // Extend root assigns to include `res_vec` (which counts to n).
        self.root.1.extend(res_vec);
    }

    /// Helper to `count_to_n`
    /// Increment when we are NOT in an offload state
    /// e.g., if `offload_states` == [2,4,6] then
    /// We should increment when !(fsm == 2 | fsm == 4 | fsm == 6).
    /// There are a couple corner cases we need to think about (in particular,
    /// we should guard the 0->1 transition differently if `incr_start_cond` is
    /// some(), and we should reset rather than increment when we are in the final
    /// fsm state).
    fn increment_if_not_offloading(
        &mut self,
        incr_start_cond: Option<ir::Guard<Nothing>>,
        offload_states: &[u64],
        adder: ir::RRC<ir::Cell>,
        parent_fsm: ir::RRC<StaticFSM>,
        builder: &mut ir::Builder,
    ) -> Vec<ir::Assignment<Nothing>> {
        let mut res_vec = vec![];
        let mut offload_state_guard: ir::Guard<Nothing> =
            ir::Guard::Not(Box::new(ir::Guard::True));
        for offload_state in offload_states {
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

        let mut incr_guard = not_offload_state;

        // If incr_start_cond.is_some(), then we have to specially take into
        // account this scenario when incrementing the FSM.
        if let Some(g) = incr_start_cond.clone() {
            // If we offload during the transition from cycle 0->1 transition
            // then we don't need a special first transition guard.
            // (we will make sure the child will add this guard when
            // it is counting to n.)
            if let Some(((beg, end), state_type)) =
                self.fsm_schedule.iter().next()
            {
                if !(matches!(state_type, StateType::Offload(_))
                    && *beg == 0
                    && *end > 1)
                {
                    let first_state = self.get_fsm_query((0, 1), builder);
                    // We must handle the 0->1 transition separately.
                    // fsm.in = fsm == 0 & incr_start_cond ? fsm + 1;
                    // fsm.write_en = fsm == 0 & incr_start_cond ? 1'd1;
                    res_vec.extend(
                        parent_fsm.borrow_mut().conditional_increment(
                            first_state.clone().and(g),
                            Rc::clone(&adder),
                            builder,
                        ),
                    );
                    // We also have to add fsm != 0 to incr_guard since
                    // we have just added assignments to handle this situation
                    // separately
                    incr_guard = incr_guard.and(first_state.not())
                }
            }
        };

        // We shouldn't increment when we are in the final state
        // (we should be resetting instead).
        // So we need to `& !in_final_state` to the guard.
        let final_fsm_state =
            self.get_fsm_query((self.latency - 1, self.latency), builder);
        let not_final_state = final_fsm_state.not();

        // However, if the final state is an offload state, then there's no need
        // to make this extra check of not being in the last state.
        if let Some((_, (_, end_final_child))) = self.children.last() {
            // If the final state is not an offload state, then
            // we need to add this check.
            if *end_final_child != self.latency {
                incr_guard = incr_guard.and(not_final_state);
            }
        } else {
            // Also, if there is just no offloading, then we need to add this check.
            incr_guard = incr_guard.and(not_final_state);
        };

        // Conditionally increment based on `incr_guard`
        res_vec.extend(parent_fsm.borrow_mut().conditional_increment(
            incr_guard,
            Rc::clone(&adder),
            builder,
        ));

        res_vec
    }

    /// Helper to `count_to_n`
    /// Increment when we ARE in an offload state, but the child being
    /// offloaded is in its final state.
    fn increment_if_child_final_state(
        &mut self,
        offload_states: &[u64],
        adder: ir::RRC<ir::Cell>,
        parent_fsm: ir::RRC<StaticFSM>,
        builder: &mut ir::Builder,
    ) -> Vec<ir::Assignment<Nothing>> {
        let mut res_vec = vec![];
        for (i, (child, (_, end))) in self
            .children
            .iter_mut()
            // If child only lasts a single cycle, then we can just unconditionally increment.
            // We handle that case above (since `offload_states` only includes children that
            // last more than one cycle).
            .filter(|(_, (beg, end))| beg + 1 != *end)
            .enumerate()
        {
            // We need to increment parent when child is in final state.
            // For example, if the parent is offloading to `child_x` when it
            // is in state 5, the guard would look like
            // fsm.in = fsm == 5 && child_x_fsm_in_final_state ? fsm + 1;
            // fsm.write_en == 5 && child_x_fsm_in_final_state ? 1'd1;

            // The one exception:
            // If the offload state is the last state (end == self.latency) then we don't
            // increment, we need to reset to 0 (which we handle separately).
            if *end != self.latency {
                // We know each offload state corresponds to exactly one child.
                let child_state = offload_states[i];
                // Checking that we are in child state, e.g., `(fsm == 5)`
                // in the above example.
                let in_child_state = parent_fsm
                    .borrow_mut()
                    .query_between(builder, (child_state, child_state + 1));
                // now we need to check `child_fsm_in_final_state`
                let total_child_latency =
                    child.get_latency() * child.get_num_repeats();
                let child_final_state = child.query_between(
                    (total_child_latency - 1, total_child_latency),
                    builder,
                );
                // Conditionally increment when `fsm==5 & child_final_state`
                let parent_fsm_incr =
                    parent_fsm.borrow_mut().conditional_increment(
                        in_child_state.and(child_final_state),
                        Rc::clone(&adder),
                        builder,
                    );
                res_vec.extend(parent_fsm_incr);
            }
        }
        res_vec
    }

    /// `Realize` each static group in the tree into a dynamic group.
    /// In particular, this involves converting %[i:j] guards into actual
    /// fsm register queries (which can get complicated with out tree structure:
    /// it's not just i <= fsm < j anymore).
    ///
    /// `reset_early_map`, `fsm_info_map`, and `group_rewrites` are all
    /// metadata to make it more easier later on when we are rewriting control,
    ///  adding wrapper groups when necessary, etc.
    fn realize(
        &mut self,
        ignore_timing_guards: bool,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        fsm_info_map: &mut HashMap<
            ir::Id,
            (ir::Id, ir::Guard<Nothing>, ir::Guard<Nothing>),
        >,
        group_rewrites: &mut ir::rewriter::PortRewriteMap,
        builder: &mut ir::Builder,
    ) {
        // Get static group we are "realizing".
        let static_group = Rc::clone(
            static_groups
                .iter()
                .find(|sgroup| sgroup.borrow().name() == self.root.0)
                .expect("couldn't find static group"),
        );
        // Create the dynamic "early reset group" that will replace the static group.
        let static_group_name = static_group.borrow().name();
        let mut early_reset_name = static_group_name.to_string();
        early_reset_name.insert_str(0, "early_reset_");
        let early_reset_group = builder.add_group(early_reset_name);

        // Realize the static %[i:j] guards to fsm queries.
        // *This is the most of the difficult thing the function does*.
        // This is significantly more complicated with a tree structure.
        let mut assigns = static_group
            .borrow()
            .assignments
            .clone()
            .into_iter()
            .map(|assign| {
                self.make_assign_dyn(
                    assign,
                    false,
                    ignore_timing_guards,
                    builder,
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

        // Adding the assignments of `self.root` (mainly the `count_to_n`
        // assignments).
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
            // If the tree does not have an fsm cell, then we can err on the
            // side of giving it its own unique identifier.
            None => self.root.0,
            Some(fsm_rc) => fsm_rc.borrow().get_unique_id(),
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
                ignore_timing_guards,
                static_groups,
                reset_early_map,
                fsm_info_map,
                group_rewrites,
                builder,
            )
        })
    }

    // Rephrasing an (i,j) query: this breaks up the guard and makes it easier
    // to figure out what logic we need to instantiate to perform the query.
    // Restructure an (i,j) query into:
    // (beg, middle, end) query.
    // This is best explained by example.
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
        Option<SingleIterQuery>,
        Option<ItersQuery>,
        Option<SingleIterQuery>,
    ) {
        // Splitting the query into an fsm query and and iteration query.
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

        // Scenario 1: the query spans only a single iteration.
        // In this case, we set beg_query to
        // `Some(<that single iteration>, (beg_fsm_query->end_fsm_query))``
        // and set middle and end to None.
        if beg_iter_query == end_iter_query {
            let repeat_query = beg_iter_query;
            let fsm_query = (beg_fsm_query, end_fsm_query);
            let res = Some((repeat_query, fsm_query));
            (res, None, None)
        }
        // Scenario 2: the query spans only 2 iterations.
        // In this case, we only need a beg_query and an end_query, but no
        // middle query.
        else if beg_iter_query + 1 == end_iter_query {
            let middle_res = None;

            let repeat_query0 = beg_iter_query;
            // We know the beg_query stretches into the next iteration,
            // so we can end it at self.latency.
            let fsm_query0 = (beg_fsm_query, self.latency);
            let beg_res = Some((repeat_query0, fsm_query0));

            let repeat_query1 = end_iter_query;
            // We know the end_query stretches backwards into the previous iteration,
            // so we can start it at 0.
            let fsm_query1 = (0, end_fsm_query);
            let end_res = Some((repeat_query1, fsm_query1));

            (beg_res, middle_res, end_res)
        }
        // Scenario 3: the query spans 3 or more iterations.
        // In this case, we need the middle_query for the middle iterations,
        // and the beg and end queries for (parts of) the
        // first and last iterations for this query.
        else {
            let mut unconditional_repeat_query =
                (beg_iter_query + 1, end_iter_query);

            let repeat_query0 = beg_iter_query;
            // We know the beg_query stretches into the next iteration,
            // so we can end it at self.latency.
            let fsm_query0 = (beg_fsm_query, self.latency);
            let mut beg_res = Some((repeat_query0, fsm_query0));
            // if beg_fsm_query == 0, then beg_query spans the entire iterations,
            // so we can just add it the unconditional_repeat_query (i.e., the middle_query).
            if beg_fsm_query == 0 {
                beg_res = None;
                unconditional_repeat_query.0 -= 1;
            }

            let repeat_query1 = end_iter_query;
            // We know the end_query stretches backwards into the previous iteration,
            // so we can start it at 0.
            let fsm_query1 = (0, end_fsm_query);
            let mut end_res = Some((repeat_query1, fsm_query1));
            // If end_fsm_query == self.latency, then end_res spans the entire iterations,
            // so we can just add it the unconditional_repeat_query (i.e., the middle_query).
            if end_fsm_query == self.latency {
                end_res = None;
                unconditional_repeat_query.1 += 1;
            }

            (beg_res, Some(unconditional_repeat_query), end_res)
        }
    }

    // Given query (i,j), get the fsm query for cycles (i,j).
    // Does NOT check the iteration number.
    // This is greatly complicated by the offloading to children.
    // We use a resturcturing that organizes the query into (beg, middle, end),
    // similar to (but not the same as) self.restructure_query().
    fn get_fsm_query(
        &mut self,
        query: (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        // If guard is true the entire execution, then return `true`.
        if 0 == query.0 && self.latency == query.1 {
            return ir::Guard::True;
        }

        let fsm_cell_opt = self.fsm_cell.as_ref();
        if fsm_cell_opt.is_none() {
            // If there is no fsm cell even though latency > 1, then we must
            // have offloaded the entire latency. Therefore we just need
            // to query the child.
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
        // Suppose fsm_schedule =    Cycles     FSM State (i.e., `fsm.out`)
        //                           (0..10) ->  Normal[0,10)
        //                           (10..30) -> Offload(10) // Offloading to child
        //                           (30..40) -> Normal[11, 21)
        //                           (40,80) ->  Offload(21)
        //                           (80,100)->  Normal[22, 42)
        // And query = (15,95).
        // Then at the end of the following `for` loop we want:
        // `beg_interval` should be fsm == 10 && <child.query_between(5,20)>
        // `middle_interval` should be (11, 22)
        // `end_interval` should be 22 <= fsm < 37
        for ((beg, end), state_type) in self.fsm_schedule.iter() {
            // Check if the query encompasses the entire interval.
            // If so, we add it to the "middle" interval.
            if query_beg <= *beg && *end <= query_end {
                // Get the interval we have to add, based on `state_type`.
                let interval_to_add = match state_type {
                    StateType::Normal(fsm_interval) => *fsm_interval,
                    StateType::Offload(offload_state) => {
                        (*offload_state, offload_state + 1)
                    }
                };
                // Add `interval_to_add` to `middle_interval`.
                match middle_interval {
                    None => middle_interval = Some(interval_to_add),
                    Some((cur_start, cur_end)) => {
                        assert!(cur_end == interval_to_add.0);
                        middle_interval = Some((cur_start, interval_to_add.1));
                    }
                }
            }
            // Otherwise check if the beginning of the query lies within the
            // interval (This should only happen once). Add to `beg_interval`.
            else if *beg <= query_beg && query_beg < *end {
                assert!(beg_interval.is_false());
                // This is the query, but relativized to the start of the current interval.
                let relative_query = (query_beg - beg, query_end - beg);
                match state_type {
                    // If we are not offloading, then we can just produce a normal
                    // query.
                    StateType::Normal((beg_fsm_interval, end_fsm_interval)) => {
                        let translated_query = (
                            beg_fsm_interval + relative_query.0,
                            // This query either stretches into the next interval, or
                            // ends within the interval: we want to capture both of these choices.
                            std::cmp::min(
                                beg_fsm_interval + relative_query.1,
                                *end_fsm_interval,
                            ),
                        );
                        beg_interval = *fsm_cell
                            .borrow_mut()
                            .query_between(builder, translated_query);
                    }
                    // If we are not offloading, then we first check the state,
                    // then we must query the corresponding child.
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
                                relative_query.0,
                                // This query either stretches into another interval, or
                                // ends within the interval: we want to capture both of these choices.
                                std::cmp::min(
                                    relative_query.1,
                                    child.get_latency()
                                        * child.get_num_repeats(),
                                ),
                            ),
                            builder,
                        );
                        beg_interval = in_offload_state.and(child_query);
                    }
                };
            }
            // Check if the end of the query lies within the
            // interval (This should only happen once) Add to `end_interval`.
            else if *beg < query_end && query_end <= *end {
                // We only need the end of the relative query.
                // If we try to get the beginning then we could get overflow error.
                let relative_query_end = query_end - beg;
                assert!(end_interval.is_false());
                match state_type {
                    StateType::Normal((beg_fsm_interval, _)) => {
                        end_interval = *fsm_cell.borrow_mut().query_between(
                            builder,
                            // This query must stretch backwards into a preiouvs interval
                            // Otherwise it would have been caught by the
                            // So beg_fsm_interval is a safe start.
                            (
                                *beg_fsm_interval,
                                beg_fsm_interval + relative_query_end,
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
                        // We know this must stretch backwards
                        // into a previous interval: otherwise, it
                        // would have been caught by the previous elif condition.
                        // therefore, we can start the child query at 0.
                        let child_query = child
                            .query_between((0, relative_query_end), builder);
                        end_interval = in_offload_state.and(child_query);
                    }
                };
            }
            if matches!(state_type, StateType::Offload(_)) {
                child_index += 1;
            }
        }

        // Turn `middle_interval` into an actual `ir::Guard`.
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

    // Produces a guard that checks whether query.0 <= self.iter_count_cell < query.1
    fn get_repeat_query(
        &mut self,
        query: (u64, u64),
        builder: &mut ir::Builder,
    ) -> Box<ir::Guard<Nothing>> {
        // If self.num_repeats == 1, then no need for a complicated query.
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

    // Produce a guard that checks:
    //   - whether iteration == repeat_query AND
    //   - whether %[fsm_query.0:fsm_query.1]
    fn check_iteration_and_fsm_state(
        &mut self,
        (repeat_query, fsm_query): (u64, (u64, u64)),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        let fsm_guard = self.get_fsm_query(fsm_query, builder);

        // Checks `self.iter_count_cell`.
        let counter_guard =
            self.get_repeat_query((repeat_query, repeat_query + 1), builder);
        ir::Guard::And(Box::new(fsm_guard), counter_guard)
    }

    // Converts a %[i:j] query into a query of `self`'s and its childrens
    // iteration registers.
    fn query_between(
        &mut self,
        query: (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        // See `restructure_query` to see what we're doing.
        // But basically:
        // beg_iter_query = Option(iteration number, cycles during that iteration the query is true).
        // middle_iter_query = Option(iterations during which the query is true the entire iteration).
        // end_iter_query = Option(iteration number, cycles during that iteration the query is true).
        let (beg_iter_query, middle_iter_query, end_iter_query) =
            self.restructure_query(query);

        // Call `check_iteration_and_fsm_state` for beg and end queries.
        let g0 = match beg_iter_query {
            None => ir::Guard::True.not(),
            Some(q0) => self.check_iteration_and_fsm_state(q0, builder),
        };
        let g1 = match end_iter_query {
            None => ir::Guard::True.not(),
            Some(q1) => self.check_iteration_and_fsm_state(q1, builder),
        };

        // Call `get_repeat_query` for middle_iter_queries.
        let rep_query = match middle_iter_query {
            None => Box::new(ir::Guard::True.not()),
            Some(rq) => self.get_repeat_query(rq, builder),
        };
        g0.or(g1.or(*rep_query))
    }

    // Takes in a static guard `guard`, and returns equivalent dynamic guard
    // The only thing that actually changes is the Guard::Info case
    // We need to turn static_timing to dynamic guards using `fsm`.
    // See `make_assign_dyn` for explanations of `global_view` and `ignore_timing`
    // variable.
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
                // If `ignore_timing` is true, then just return a true guard.
                if ignore_timing {
                    assert!(static_timing.get_interval() == (0, 1));
                    return Box::new(ir::Guard::True);
                }
                if global_view {
                    // For global_view we call `query_between`
                    Box::new(
                        self.query_between(
                            static_timing.get_interval(),
                            builder,
                        ),
                    )
                } else {
                    // For local_view we call `get_fsm_query`
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

    /// Takes in static assignment `assign` and returns a dynamic assignments
    /// For example, it could transform the guard %[2:3] -> fsm.out >= 2 & fsm.out <= 3
    /// `global_view`: are you just querying for a given iteration, or are
    /// you querying for the entire tree's execution?
    ///   - if `global_view` is true, then you have to include the iteration
    ///     count register in the assignment's guard.
    ///   - if `global_view` is false, then you dont' have to include it
    ///
    /// `ignore_timing`: remove static timing guards instead of transforming them
    /// into an FSM query. Note that in order to do this, the timing guard must
    /// equal %[0:1], otherwise we will throw an error. This option is here
    /// mainly to save resource usage.
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

    // Helper function: checks
    // whether the tree offloads its entire latency, and returns the
    // appropriate `bool`.
    fn offload_entire_latency(&self) -> bool {
        self.children.len() == 1
            && self
                .children
                .iter()
                .any(|(_, (beg, end))| *beg == 0 && *end == self.latency)
                // This last check is prob unnecessary since it follows from the first two.
            && self.num_states == 1
    }
}

/// These methods handle adding conflicts to the tree (to help coloring for
/// sharing FSMs)
impl SingleNode {
    // Get names of groups corresponding to all nodes
    pub fn get_all_nodes(&self) -> Vec<ir::Id> {
        let mut res = vec![self.root.0];
        for (child, _) in &self.children {
            res.extend(child.get_all_nodes())
        }
        res
    }

    // Adds conflicts between children and any descendents.
    // Also add conflicts between any overlapping children. XXX(Caleb): normally
    // there shouldn't be overlapping children, but when we are doing the traditional
    // method in we don't offload (and therefore don't need this tree structure)
    // I have created dummy trees for the sole purpose of drawing conflicts
    pub fn add_conflicts(&self, conflict_graph: &mut GraphColoring<ir::Id>) {
        let root_name = self.root.0;
        for (child, _) in &self.children {
            for sgroup in &child.get_all_nodes() {
                conflict_graph.insert_conflict(&root_name, sgroup);
            }
            child.add_conflicts(conflict_graph);
        }
        // Adding conflicts between overlapping children.
        for ((child_a, (beg_a, end_a)), (child_b, (beg_b, end_b))) in
            self.children.iter().tuple_combinations()
        {
            // Checking if children overlap: either b begins within a, it
            // ends within a, or it encompasses a's entire interval.
            if ((beg_a <= beg_b) & (beg_b < end_a))
                | ((beg_a < end_b) & (end_b <= end_a))
                | (beg_b <= beg_a && end_a <= end_b)
            {
                // Adding conflicts between all nodes of the children if
                // the children overlap.
                for a_node in child_a.get_all_nodes() {
                    for b_node in child_b.get_all_nodes() {
                        conflict_graph.insert_conflict(&a_node, &b_node);
                    }
                }
            }
        }
    }

    // Gets max value according to some function f.
    pub fn get_max_value<F>(&self, name: &ir::Id, f: &F) -> u64
    where
        F: Fn(&SingleNode) -> u64,
    {
        let mut cur_max = 0;
        if self.root.0 == name {
            cur_max = std::cmp::max(cur_max, f(self));
        }
        for (child, _) in &self.children {
            cur_max = std::cmp::max(cur_max, child.get_max_value(name, f));
        }
        cur_max
    }
}

/// Represents a group of `Nodes` that execute in parallel.
pub struct ParNodes {
    /// Name of the `par_group` that fires off the threads
    pub group_name: ir::Id,
    /// Latency
    pub latency: u64,
    /// Num Repeats
    pub num_repeats: u64,
    /// (Thread, interval thread is active).
    /// Interval thread is active should always start at 0.
    pub threads: Vec<(Node, (u64, u64))>,
}

impl ParNodes {
    /// Instantiates FSMs by recursively instantiating FSM for each thread.
    pub fn instantiate_fsms(
        &mut self,
        builder: &mut ir::Builder,
        coloring: &HashMap<ir::Id, ir::Id>,
        colors_to_max_values: &HashMap<ir::Id, (u64, u64)>,
        colors_to_fsm: &mut HashMap<
            ir::Id,
            (OptionalStaticFSM, OptionalStaticFSM),
        >,
        one_hot_cutoff: u64,
    ) {
        for (thread, _) in &mut self.threads {
            thread.instantiate_fsms(
                builder,
                coloring,
                colors_to_max_values,
                colors_to_fsm,
                one_hot_cutoff,
            );
        }
    }

    /// Counts to N by recursively calling `count_to_n` on each thread.
    pub fn count_to_n(
        &mut self,
        builder: &mut ir::Builder,
        incr_start_cond: Option<ir::Guard<Nothing>>,
    ) {
        for (thread, _) in &mut self.threads {
            thread.count_to_n(builder, incr_start_cond.clone());
        }
    }

    /// Realizes static groups into dynamic group.
    pub fn realize(
        &mut self,
        ignore_timing_guards: bool,
        static_groups: &Vec<ir::RRC<ir::StaticGroup>>,
        reset_early_map: &mut HashMap<ir::Id, ir::Id>,
        fsm_info_map: &mut HashMap<
            ir::Id,
            (ir::Id, ir::Guard<Nothing>, ir::Guard<Nothing>),
        >,
        group_rewrites: &mut ir::rewriter::PortRewriteMap,
        builder: &mut ir::Builder,
    ) {
        // Get static grouo we are "realizing".
        let static_group = Rc::clone(
            static_groups
                .iter()
                .find(|sgroup| sgroup.borrow().name() == self.group_name)
                .expect("couldn't find static group"),
        );
        // Create the dynamic "early reset group" that will replace the static group.
        let static_group_name = static_group.borrow().name();
        let mut early_reset_name = static_group_name.to_string();
        early_reset_name.insert_str(0, "early_reset_");
        let early_reset_group = builder.add_group(early_reset_name);

        // Get the longest node.
        let longest_node = self.get_longest_node();

        // If one thread lasts 10 cycles, and another lasts 5 cycles, then the par_group
        // will look like this:
        // static<10> group par_group {
        //   thread1[go] = 1'd1;
        //   thread2[go] = %[0:5] ? 1'd1;
        // }
        // Therefore the %[0:5] needs to be realized using the FSMs from thread1 (the
        // longest FSM).
        let mut assigns = static_group
            .borrow()
            .assignments
            .clone()
            .into_iter()
            .map(|assign| {
                longest_node.make_assign_dyn(
                    assign,
                    true,
                    ignore_timing_guards,
                    builder,
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

        let fsm_identifier = match longest_node.fsm_cell.as_ref() {
            // If the tree does not have an fsm cell, then we can err on the
            // side of giving it its own unique identifier.
            None => longest_node.root.0,
            Some(fsm_rc) => fsm_rc.borrow().get_unique_id(),
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
        self.threads.iter_mut().for_each(|(child, _)| {
            child.realize(
                ignore_timing_guards,
                static_groups,
                reset_early_map,
                fsm_info_map,
                group_rewrites,
                builder,
            )
        })
    }

    /// Recursively searches each thread to get the longest (in terms of
    /// cycle counts) SingleNode.
    pub fn get_longest_node(&mut self) -> &mut SingleNode {
        let max = self.threads.iter_mut().max_by_key(|(child, _)| {
            (child.get_latency() * child.get_num_repeats()) as i64
        });
        if let Some((max_child, _)) = max {
            match max_child {
                Node::Par(par_nodes) => par_nodes.get_longest_node(),
                Node::Single(single_node) => single_node,
            }
        } else {
            unreachable!("self.children is empty/no maximum value found");
        }
    }

    /// Use the longest node to query between.
    pub fn query_between(
        &mut self,
        query: (u64, u64),
        builder: &mut ir::Builder,
    ) -> ir::Guard<Nothing> {
        let longest_node = self.get_longest_node();
        longest_node.query_between(query, builder)
    }
}

/// Used to add conflicts for graph coloring for sharing FSMs.
/// See the equivalent SingleNode implementation for more details.
impl ParNodes {
    pub fn get_all_nodes(&self) -> Vec<ir::Id> {
        let mut res = vec![];
        for (thread, _) in &self.threads {
            res.extend(thread.get_all_nodes())
        }
        res
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
        F: Fn(&SingleNode) -> u64,
    {
        let mut cur_max = 0;
        for (thread, _) in &self.threads {
            cur_max = std::cmp::max(cur_max, thread.get_max_value(name, f));
        }
        cur_max
    }
}
