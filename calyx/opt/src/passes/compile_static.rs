use crate::analysis::{
    GraphColoring, Node, ParNodes, SingleNode, StateType, StaticFSM,
};
use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir, Nothing, PortParent};
use calyx_ir::{GetAttributes, guard, structure};
use calyx_utils::{CalyxResult, Error};
use core::panic;
use ir::{RRC, build_assignments};
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Not;
use std::rc::Rc;
use std::vec;

type OptionalStaticFSM = Option<ir::RRC<StaticFSM>>;

/// Compiles Static Islands
pub struct CompileStatic {
    /// maps original static group names to the corresponding group that has an FSM that reset early
    reset_early_map: HashMap<ir::Id, ir::Id>,
    /// maps group that has an FSM that resets early to its dynamic "wrapper" group name.
    wrapper_map: HashMap<ir::Id, ir::Id>,
    /// maps fsm names to their corresponding signal_reg
    signal_reg_map: HashMap<ir::Id, ir::Id>,
    /// maps reset_early_group names to (fsm_identifier, fsm_first_state, final_fsm_state);
    /// The "fsm identifier" is just the name of the fsm (if it exists) and
    /// some other unique identifier if it doesn't exist (this works because
    /// it is always fine to give each entry group its own completely unique identifier.)
    fsm_info_map:
        HashMap<ir::Id, (ir::Id, ir::Guard<Nothing>, ir::Guard<Nothing>)>,
    /// Maps `static_group[go]` to `early_reset_group[go]`.
    group_rewrites: ir::rewriter::PortRewriteMap,

    /// Command line arguments:
    /// Cutoff for one hot encoding. Anything larger than the cutoff becomes
    /// binary.
    one_hot_cutoff: u64,
    /// Bool indicating whether to make the FSM pause (i.e., stop counting) when
    /// offloading. In order for compilation to make sense, this parameter must
    /// match the parameter for `static-inline`.
    offload_pause: bool,
    /// Bool indicating whether to greedily share the FSM registers
    greedy_share: bool,
}

impl Named for CompileStatic {
    fn name() -> &'static str {
        "compile-static"
    }

    fn description() -> &'static str {
        "compiles static sub-programs into a dynamic group"
    }

    fn opts() -> Vec<PassOpt> {
        vec![PassOpt::new(
            "one-hot-cutoff",
            "The upper limit on the number of states the static FSM must have before we pick binary
            encoding over one-hot. Defaults to 0 (i.e., always choose binary encoding)",
            ParseVal::Num(0),
            PassOpt::parse_num,
        ),
        PassOpt::new(
            "offload-pause",
            "Whether to pause the static FSM when offloading. Note that this
            parameter must be in sync with the static-inliner's offload-pause
            parameter for compilation to work correctly",
            ParseVal::Bool(true),
            PassOpt::parse_bool,
        ),
        PassOpt::new(
            "greedy-share",
            "Whether to greedily share the FSMs",
            ParseVal::Bool(true),
            PassOpt::parse_bool,
        )

        ]
    }
}

impl ConstructVisitor for CompileStatic {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let opts = Self::get_opts(ctx);

        Ok(CompileStatic {
            one_hot_cutoff: opts["one-hot-cutoff"].pos_num().unwrap(),
            offload_pause: opts["offload-pause"].bool(),
            greedy_share: opts["greedy-share"].bool(),
            reset_early_map: HashMap::new(),
            wrapper_map: HashMap::new(),
            signal_reg_map: HashMap::new(),
            fsm_info_map: HashMap::new(),
            group_rewrites: ir::rewriter::PortRewriteMap::default(),
        })
    }

    fn clear_data(&mut self) {
        self.reset_early_map = HashMap::new();
        self.wrapper_map = HashMap::new();
        self.signal_reg_map = HashMap::new();
        self.fsm_info_map = HashMap::new();
        self.group_rewrites = ir::rewriter::PortRewriteMap::default();
    }
}

impl CompileStatic {
    /// Builds a wrapper group for group named group_name using fsm_final_state
    /// and a signal_reg.
    /// We set the signal_reg high on the final fsm state, since we know the
    /// `done` signal should be high the next cycle after that.
    /// `add_resetting_logic` is a bool; since the same FSM/signal_reg pairing
    /// may be used for multiple static islands, and we only add resetting logic
    /// for the signal_reg once.
    fn build_wrapper_group(
        fsm_final_state: ir::Guard<Nothing>,
        group_name: &ir::Id,
        signal_reg: ir::RRC<ir::Cell>,
        builder: &mut ir::Builder,
        add_reseting_logic: bool,
    ) -> ir::RRC<ir::Group> {
        // Get the group and fsm necessary to build the wrapper group.
        let early_reset_group = builder
            .component
            .get_groups()
            .find(*group_name)
            .unwrap_or_else(|| {
                unreachable!(
                    "called build_wrapper_group with {}, which is not a group",
                    group_name
                )
            });

        structure!( builder;
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
        );
        // Making the rest of the guards guards:
        // signal_reg.out
        let signal_reg_guard: ir::Guard<ir::Nothing> =
            guard!(signal_reg["out"]);
        // !signal_reg.out
        let not_signal_reg = signal_reg_guard.clone().not();
        // <fsm in final state> & ! signal_reg.out
        let final_state_not_signal = fsm_final_state & not_signal_reg;

        // create the wrapper group for early_reset_group
        let mut wrapper_name = group_name.clone().to_string();
        wrapper_name.insert_str(0, "wrapper_");
        let g = builder.add_group(wrapper_name);
        let group_assigns = build_assignments!(
          builder;
          // early_reset_group[go] = 1'd1
          early_reset_group["go"] = ? signal_on["out"];
          // when <fsm_in_final_state> and !signal_reg, then set signal_reg to high
          signal_reg["write_en"] = final_state_not_signal ? signal_on["out"];
          signal_reg["in"] =  final_state_not_signal ? signal_on["out"];
          // group[done] = signal_reg.out ? 1'd1
          g["done"] = signal_reg_guard ? signal_on["out"];
        );
        if add_reseting_logic {
            // continuous assignments to reset signal_reg back to 0 when the wrapper is done
            let continuous_assigns = build_assignments!(
                builder;
                // when (fsm == 0 & signal_reg is high), which is the done condition of the wrapper,
                // reset the signal_reg back to low
                signal_reg["write_en"] = signal_reg_guard ? signal_on["out"];
                signal_reg["in"] =  signal_reg_guard ? signal_off["out"];
            );
            builder.add_continuous_assignments(continuous_assigns.to_vec());
        }
        g.borrow_mut().assignments = group_assigns.to_vec();
        g.borrow_mut().attributes =
            early_reset_group.borrow().attributes.clone();
        g
    }

    /// compile `while` whose body is `static` control such that at the end of each
    /// iteration, the checking of condition does not incur an extra cycle of
    /// latency.
    /// We do this by wrapping the early reset group of the body with
    /// another wrapper group, which sets the go signal of the early reset group
    /// high, and is done when at the 0th cycle of each iteration, the condtion
    /// port is done.
    /// Note: this only works if the port for the while condition is `@stable`.
    fn build_wrapper_group_while(
        &self,
        fsm_first_state: ir::Guard<Nothing>,
        group_name: &ir::Id,
        port: RRC<ir::Port>,
        builder: &mut ir::Builder,
    ) -> RRC<ir::Group> {
        let reset_early_group = builder
            .component
            .find_group(*group_name)
            .unwrap_or_else(|| {
                unreachable!(
                    "called build_wrapper_group with {}, which is not a group",
                    group_name
                )
            });

        let wrapper_group =
            builder.add_group(format!("while_wrapper_{group_name}"));

        structure!(
            builder;
            let one = constant(1, 1);
        );

        let port_parent = port.borrow().cell_parent();
        let port_name = port.borrow().name;
        let done_guard = guard!(port_parent[port_name]).not() & fsm_first_state;

        let assignments = build_assignments!(
            builder;
            reset_early_group["go"] = ? one["out"];
            wrapper_group["done"] = done_guard ? one["out"];
        );

        wrapper_group.borrow_mut().assignments.extend(assignments);
        wrapper_group
    }

    // Get early reset group name from static control (we assume the static control
    // is an enable).
    fn get_reset_group_name(&self, sc: &mut ir::StaticControl) -> &ir::Id {
        // assume that there are only static enables left.
        // if there are any other type of static control, then error out.
        let ir::StaticControl::Enable(s) = sc else {
            unreachable!(
                "Non-Enable Static Control should have been compiled away. Run {} to do this",
                crate::passes::StaticInliner::name()
            );
        };

        let sgroup = s.group.borrow_mut();
        let sgroup_name = sgroup.name();
        // get the "early reset group". It should exist, since we made an
        // early_reset group for every static group in the component

        (self.reset_early_map.get(&sgroup_name).unwrap_or_else(|| {
            unreachable!("group {} not in self.reset_early_map", sgroup_name)
        })) as _
    }
}

// These are the functions used to allocate FSMs to static islands through a
// greedy coloring algorithm.
impl CompileStatic {
    // Given a list of `static_groups`, find the group named `name`.
    // If there is no such group, then there is an unreachable! error.
    fn find_static_group(
        name: &ir::Id,
        static_groups: &[ir::RRC<ir::StaticGroup>],
    ) -> ir::RRC<ir::StaticGroup> {
        Rc::clone(
            static_groups
                .iter()
                .find(|static_group| static_group.borrow().name() == name)
                .unwrap_or_else(|| {
                    unreachable!("couldn't find static group {name}")
                }),
        )
    }

    /// Add conflicts between all nodes of `fsm_trees` which are executing
    /// on separate threads of a dynamic `par` block.
    /// This function adds conflicts between nodes of separate trees.
    fn add_par_conflicts(
        c: &ir::Control,
        fsm_trees: &Vec<Node>,
        conflict_graph: &mut GraphColoring<ir::Id>,
    ) {
        match c {
            ir::Control::Empty(_)
            | ir::Control::Enable(_)
            | ir::Control::Invoke(_)
            | ir::Control::Static(_) => (),
            ir::Control::Seq(seq) => {
                for stmt in &seq.stmts {
                    Self::add_par_conflicts(stmt, fsm_trees, conflict_graph);
                }
            }
            ir::Control::Repeat(ir::Repeat { body, .. })
            | ir::Control::While(ir::While { body, .. }) => {
                Self::add_par_conflicts(body, fsm_trees, conflict_graph)
            }
            ir::Control::If(if_stmt) => {
                Self::add_par_conflicts(
                    &if_stmt.tbranch,
                    fsm_trees,
                    conflict_graph,
                );
                Self::add_par_conflicts(
                    &if_stmt.fbranch,
                    fsm_trees,
                    conflict_graph,
                );
            }
            ir::Control::Par(par) => {
                // `sgroup_conflict_vec` is a vec of HashSets.
                // Each item in the vec corresponds to a par thread, and holds
                // all of the groups executed in that thread.
                let mut sgroup_conflict_vec: Vec<HashSet<ir::Id>> = Vec::new();
                for stmt in &par.stmts {
                    sgroup_conflict_vec.push(HashSet::from_iter(
                        Self::get_static_enables(stmt),
                    ));
                }
                for (thread1_st_enables, thread2_st_enables) in
                    sgroup_conflict_vec.iter().tuple_combinations()
                {
                    // For each static group g1 enabled in thread1 and static
                    // group g2 enabled in thread2 respectively, add a conflict
                    // each node in g1 and g2's corresponding trees.
                    for static_enable1 in thread1_st_enables {
                        for static_enable2 in thread2_st_enables {
                            // Getting tree1
                            let tree1 = fsm_trees
                                .iter()
                                .find(|tree| {
                                    tree.get_group_name() == static_enable1
                                })
                                .expect("couldn't find FSM tree");
                            // Getting tree2
                            let tree2 = fsm_trees
                                .iter()
                                .find(|tree| {
                                    tree.get_group_name() == static_enable2
                                })
                                .expect("couldn't find tree");
                            // Add conflict between each node in tree1 and tree2
                            for sgroup1 in tree1.get_all_nodes() {
                                for sgroup2 in tree2.get_all_nodes() {
                                    conflict_graph
                                        .insert_conflict(&sgroup1, &sgroup2)
                                }
                            }
                        }
                    }
                }
                // Necessary to add conflicts between nested pars
                for stmt in &par.stmts {
                    Self::add_par_conflicts(stmt, fsm_trees, conflict_graph);
                }
            }
            ir::Control::FSMEnable(_) => {
                todo!("should not encounter fsm nodes")
            }
        }
    }

    // Gets the maximum number of repeats for the static group named
    // `sgroup` among all trees in `tree_objects`. Most of the time, `sgroup`
    // will only appear once but it is possible that the same group appears
    // in more than one tree.
    fn get_max_num_repeats(sgroup: ir::Id, tree_objects: &Vec<Node>) -> u64 {
        let mut cur_max = 1;
        for tree in tree_objects {
            cur_max = std::cmp::max(
                cur_max,
                tree.get_max_value(&sgroup, &(|tree| tree.num_repeats)),
            )
        }
        cur_max
    }

    // Gets the maximum number of repeats for the static group named
    // `sgroup` among all trees in `tree_objects`. Most of the time, `sgroup`
    // will only appear once but it is possible that the same group appears
    // in more than one tree.
    fn get_max_num_states(sgroup: ir::Id, tree_objects: &Vec<Node>) -> u64 {
        let mut cur_max = 1;
        for tree in tree_objects {
            cur_max = std::cmp::max(
                cur_max,
                tree.get_max_value(&sgroup, &(|tree| tree.num_states)),
            )
        }
        cur_max
    }

    /// Creates a graph (one node per item in `sgroup` where nodes are the `sgroup`'s
    /// names).
    /// Use `tree_objects` and `control` to draw conflicts between any two nodes
    /// that could be executing in parallel, and returns a greedy coloring of the
    /// graph.
    pub fn get_coloring(
        &self,
        tree_objects: &Vec<Node>,
        sgroups: &[ir::RRC<ir::StaticGroup>],
        control: &mut ir::Control,
    ) -> HashMap<ir::Id, ir::Id> {
        if !self.greedy_share {
            // If !greedy_share just give each sgroup its own color.
            return sgroups
                .iter()
                .map(|g| (g.borrow().name(), g.borrow().name()))
                .collect();
        }
        let mut conflict_graph: GraphColoring<ir::Id> =
            GraphColoring::from(sgroups.iter().map(|g| g.borrow().name()));

        // Necessary conflicts to ensure correctness

        // Self::add_par_conflicts adds necessary conflicts between all nodes of
        // trees that execute in separate threads of the same `par` block: this is
        // adding conflicts between nodes of separate trees.
        Self::add_par_conflicts(control, tree_objects, &mut conflict_graph);
        for tree in tree_objects {
            // tree.add_conflicts adds the necessary conflicts within nodes of
            // same tree.
            tree.add_conflicts(&mut conflict_graph);
        }
        // Optional conflicts to ?potentially? improve QoR
        // for (sgroup1, sgroup2) in sgroups.iter().tuple_combinations() {
        //     let max_num_states1 =
        //         Self::get_max_num_states(sgroup1.borrow().name(), tree_objects);
        //     let max_num_repeats1 = Self::get_max_num_repeats(
        //         sgroup1.borrow().name(),
        //         tree_objects,
        //     );
        //     let max_num_states2 =
        //         Self::get_max_num_states(sgroup2.borrow().name(), tree_objects);
        //     let max_num_repeats2 = Self::get_max_num_repeats(
        //         sgroup2.borrow().name(),
        //         tree_objects,
        //     );
        //     if ((max_num_states1 == 1) != (max_num_states2 == 1))
        //         || ((max_num_repeats1) != (max_num_repeats2))
        //     {
        //         conflict_graph.insert_conflict(
        //             &sgroup1.borrow().name(),
        //             &sgroup2.borrow().name(),
        //         );
        //     }
        // }

        conflict_graph.color_greedy(None, true)
    }

    /// Given a coloring of group names, returns a Hashmap that maps:
    /// colors -> (max num states for that color, max num repeats for color).
    pub fn get_color_max_values(
        coloring: &HashMap<ir::Id, ir::Id>,
        tree_objects: &Vec<Node>,
    ) -> HashMap<ir::Id, (u64, u64)> {
        let mut colors_to_sgroups: HashMap<ir::Id, Vec<ir::Id>> =
            HashMap::new();
        // "Reverse" the coloring: instead of maping group names->colors,
        // map colors -> group names.
        for (group_name, color) in coloring {
            colors_to_sgroups
                .entry(*color)
                .or_default()
                .push(*group_name);
        }
        colors_to_sgroups
            .into_iter()
            .map(|(name, colors_sgroups)| {
                // Get max num states for this color
                let max_num_states = colors_sgroups
                    .iter()
                    .map(|gname| Self::get_max_num_states(*gname, tree_objects))
                    .max()
                    .expect("color is empty");
                // Get max num repeats for this color
                let max_num_repeats = colors_sgroups
                    .iter()
                    .map(|gname| {
                        Self::get_max_num_repeats(*gname, tree_objects)
                    })
                    .max()
                    .expect("color is empty");
                (name, (max_num_states, max_num_repeats))
            })
            .collect()
    }
}

impl CompileStatic {
    /// `get_interval_from_guard` returns the interval found within guard `g`.
    /// The tricky part is that sometimes there can be an implicit latency
    /// `lat` that is not explicitly stated (i.e., every assignment in a
    /// group with latency n has an implicit guard of %[0:n]). `lat` is `n`.
    fn get_interval_from_guard(
        g: &ir::Guard<ir::StaticTiming>,
        lat: u64,
    ) -> (u64, u64) {
        match g {
            calyx_ir::Guard::Info(static_timing_interval) => {
                static_timing_interval.get_interval()
            }
            calyx_ir::Guard::Not(_)
            | calyx_ir::Guard::CompOp(_, _, _)
            | calyx_ir::Guard::Port(_)
            | calyx_ir::Guard::True => (0, lat),
            calyx_ir::Guard::And(l, r) => {
                let ((beg1, end1), (beg2, end2)) = (
                    Self::get_interval_from_guard(l, lat),
                    Self::get_interval_from_guard(r, lat),
                );
                assert!(end1 - beg1 == lat || end2 - beg2 == lat);
                if end1 - beg1 == lat {
                    (beg2, end2)
                } else {
                    (beg1, end1)
                }
            }
            ir::Guard::Or(_, _) => unreachable!(
                "Shouldn't try to get interval from guard if there is an 'or' in the guard"
            ),
        }
    }

    // Given a children_sched (a sorted vec of intervals for which
    // the children are active), builds an FSM schedule and returns it,
    // along with the number of states the resulting FSM will have (42 in the
    // example given below).
    // Schedule maps cycles (i,j) -> fsm state type (i.e., what the fsm outputs).
    // Here is an example FSM schedule:
    //                           Cycles     FSM State (i.e., `fsm.out`)
    //                           (0..10) ->  Normal[0,10) // FSM counting from 0..10
    //                           (10..30) -> Offload(10) // Offloading to child
    //                           (30..40) -> Normal[11, 21)
    //                           (40,80) ->  Offload(21)
    //                           (80,100)->  Normal[22, 42)
    //
    // `target_latency` is the latency of the entire tree (100 in this case).
    fn build_tree_schedule(
        children_sched: &[(u64, u64)],
        target_latency: u64,
    ) -> (BTreeMap<(u64, u64), StateType>, u64) {
        let mut fsm_schedule = BTreeMap::new();
        let mut cur_num_states = 0;
        let mut cur_lat = 0;
        for (beg, end) in children_sched {
            // Filling in the gap between children, if necessary with a
            // `Normal` StateType.
            if cur_lat != *beg {
                fsm_schedule.insert(
                    (cur_lat, *beg),
                    StateType::Normal((
                        cur_num_states,
                        cur_num_states + (beg - cur_lat),
                    )),
                );
                cur_num_states += beg - cur_lat;
                // cur_lat = *beg; assignment is unnecessary
            }
            // Inserting an Offload StateType to the schedule.
            fsm_schedule
                .insert((*beg, *end), StateType::Offload(cur_num_states));
            cur_lat = *end;
            cur_num_states += 1;
        }
        // Filling in the gap between the final child and the end of the group
        // with a Normal StateType.
        if cur_lat != target_latency {
            fsm_schedule.insert(
                (cur_lat, target_latency),
                StateType::Normal((
                    cur_num_states,
                    cur_num_states + (target_latency - cur_lat),
                )),
            );
            cur_num_states += target_latency - cur_lat;
        }
        (fsm_schedule, cur_num_states)
    }

    /// Given a static group `target_name` and vec of `static_groups`, builds a
    /// `tree_object` for group `target_name` that repeats itself `num_repeat`
    /// times.
    fn build_tree_object(
        target_name: ir::Id,
        static_groups: &[ir::RRC<ir::StaticGroup>],
        num_repeats: u64,
    ) -> Node {
        // Find the group that will serve as the root of the tree.
        let target_group = static_groups
            .iter()
            .find(|sgroup| sgroup.borrow().name() == target_name)
            .unwrap();
        // Children of the root of the tree.
        let mut children_vec = vec![];

        let target_group_ref = target_group.borrow();
        for assign in &target_group_ref.assignments {
            // Looking for static_child[go] = %[i:j] ? 1'd1; to build children.
            // This lets us know that `static_child` is executing from cycles
            // i through j.
            match &assign.dst.borrow().parent {
                PortParent::Cell(_) => {
                    if target_group_ref.attributes.has(ir::BoolAttr::ParCtrl) {
                        panic!("")
                    }
                }
                PortParent::Group(_) | PortParent::FSM(_) => panic!(""),
                PortParent::StaticGroup(sgroup) => {
                    assert!(assign.src.borrow().is_constant(1, 1));
                    let (beg, end) = Self::get_interval_from_guard(
                        &assign.guard,
                        target_group.borrow().get_latency(),
                    );
                    let name: calyx_ir::Id = sgroup.upgrade().borrow().name();
                    // Need the following lines to determine `num_repeats`
                    // for the child.
                    let target_child_latency =
                        Self::get_sgroup_latency(name, static_groups);
                    let child_execution_time = end - beg;
                    assert!(
                        child_execution_time % target_child_latency == 0,
                        "child will execute only part of an iteration"
                    );
                    let child_num_repeats =
                        child_execution_time / target_child_latency;
                    // Recursively build a tree for the child.
                    children_vec.push((
                        Self::build_tree_object(
                            name,
                            static_groups,
                            child_num_repeats,
                        ),
                        (beg, end),
                    ));
                }
            }
        }

        if target_group_ref.attributes.has(ir::BoolAttr::ParCtrl) {
            // If we are in a par group, then the "children" are actually
            // threads that should all start at 0.
            assert!(children_vec.iter().all(|(_, (beg, _))| *beg == 0));
            Node::Par(ParNodes {
                group_name: target_name,
                threads: children_vec,
                latency: target_group_ref.latency,
                num_repeats,
            })
        } else {
            // If we are in a regular group, then the children should be
            // non-overlapping.
            children_vec.sort_by_key(|(_, interval)| *interval);
            assert!(Self::are_ranges_non_overlapping(&children_vec));
            let (fsm_schedule, num_states) = Self::build_tree_schedule(
                &children_vec
                    .iter()
                    .map(|(_, interval)| *interval)
                    .collect_vec(),
                target_group_ref.latency,
            );
            Node::Single(SingleNode {
                latency: target_group_ref.latency,
                fsm_cell: None,
                iter_count_cell: None,
                root: (target_name, vec![]),
                fsm_schedule,
                children: children_vec,
                num_repeats,
                num_states,
            })
        }
    }

    /// Builds a dummy tree, solely for the purposes of determining conflicts
    /// so we can greedily color when assigning FSMs. This should only be occuring
    /// when we count during offloading (i.e., don't pause).
    /// This tree should never actually be turned into hardware!! (Thd trees that we
    /// build here do not make sense if you want to actually do that.)
    /// We can't call `build_tree_object` on this because that looks for the
    /// `par` attribute, which isn't present when we're counting.
    fn build_dummy_tree(
        target_name: ir::Id,
        static_groups: &[ir::RRC<ir::StaticGroup>],
    ) -> Node {
        // Find the group that will serve as the root of the tree.
        let target_group = static_groups
            .iter()
            .find(|sgroup| sgroup.borrow().name() == target_name)
            .unwrap();
        let mut children_vec = vec![];
        let target_group_ref = target_group.borrow();
        assert!(
            !target_group_ref.attributes.has(ir::BoolAttr::ParCtrl),
            "ParCtrl attribute is not compatible with building dummy trees"
        );
        for assign in &target_group_ref.assignments {
            // Looking for static_child[go] = %[i:j] ? 1'd1; to build children.
            match &assign.dst.borrow().parent {
                PortParent::Cell(_) => (),
                PortParent::Group(_) | PortParent::FSM(_) => unreachable!(""),
                PortParent::StaticGroup(sgroup) => {
                    assert!(assign.src.borrow().is_constant(1, 1));
                    let (beg, end) = Self::get_interval_from_guard(
                        &assign.guard,
                        target_group.borrow().get_latency(),
                    );

                    let name: calyx_ir::Id = sgroup.upgrade().borrow().name();
                    children_vec.push((
                        Self::build_dummy_tree(name, static_groups),
                        (beg, end),
                    ));
                }
            }
        }

        children_vec.sort_by_key(|(_, interval)| *interval);
        Node::Single(SingleNode {
            latency: target_group_ref.latency,
            fsm_cell: None,
            iter_count_cell: None,
            root: (target_name, vec![]),
            fsm_schedule: BTreeMap::new(),
            children: children_vec,
            num_repeats: 1,
            num_states: target_group_ref.latency,
        })
    }

    /// Builds "trees" but just make them single nodes that never offload.
    /// This is the original strategy that we used.
    fn build_single_node(
        name: ir::Id,
        static_groups: &[ir::RRC<ir::StaticGroup>],
    ) -> Node {
        let target_group = static_groups
            .iter()
            .find(|sgroup| sgroup.borrow().name() == name)
            .unwrap();
        let target_group_ref = target_group.borrow();
        assert!(
            !target_group_ref.attributes.has(ir::BoolAttr::ParCtrl),
            "ParCtrl attribute is not compatible with building a single node"
        );

        Node::Single(SingleNode {
            latency: target_group_ref.latency,
            fsm_cell: None,
            iter_count_cell: None,
            root: (name, vec![]),
            fsm_schedule: vec![(
                (0, target_group_ref.latency),
                StateType::Normal((0, target_group_ref.latency)),
            )]
            .into_iter()
            .collect(),
            children: vec![],
            num_repeats: 1,
            num_states: target_group_ref.latency,
        })
    }

    /// Search through `static_groups` and get latency of sgroup named `name`
    fn get_sgroup_latency(
        name: ir::Id,
        static_groups: &[ir::RRC<ir::StaticGroup>],
    ) -> u64 {
        static_groups
            .iter()
            .find(|sgroup| sgroup.borrow().name() == name)
            .expect("couldn't find static group")
            .borrow()
            .get_latency()
    }

    // Given a vec of tuples (i,j) sorted by the first element (i.e., `i`) checks
    // whether the ranges do not overlap.
    fn are_ranges_non_overlapping(ranges: &[(Node, (u64, u64))]) -> bool {
        if ranges.is_empty() {
            return true;
        }
        for i in 0..ranges.len() - 1 {
            let (_, (_, end1)) = ranges[i];
            let (_, (start2, _)) = ranges[i + 1];
            // Ensure that the current range's end is less than or equal to the next range's start
            if end1 > start2 {
                return false;
            }
        }
        true
    }

    // Get a vec of all static groups that were "enabled" in `ctrl`.
    fn get_static_enables(ctrl: &ir::Control) -> Vec<ir::Id> {
        match ctrl {
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => stmts
                .iter()
                .flat_map(Self::get_static_enables)
                .collect_vec(),
            ir::Control::Empty(_)
            | ir::Control::Enable(_)
            | ir::Control::Invoke(_) => vec![],
            ir::Control::If(c) => {
                let mut tbranch_res = Self::get_static_enables(&c.tbranch);
                let fbranch_res = Self::get_static_enables(&c.fbranch);
                tbranch_res.extend(fbranch_res);
                tbranch_res
            }
            ir::Control::Repeat(ir::Repeat { body, .. })
            | ir::Control::While(ir::While { body, .. }) => {
                Self::get_static_enables(body)
            }
            ir::Control::Static(sc) => {
                let ir::StaticControl::Enable(s) = sc else {
                    unreachable!(
                        "Non-Enable Static Control should have been compiled away. Run {} to do this",
                        crate::passes::StaticInliner::name()
                    );
                };
                vec![s.group.borrow().name()]
            }
            ir::Control::FSMEnable(_) => {
                todo!("should not encounter fsm nodes")
            }
        }
    }
}

// These are the functions used to compile for the static *component* interface,
// which (annoyingly) only needs a go signal fro %0, compared to %[0:n] for
// static groups.
impl CompileStatic {
    // Used for guards in a one cycle static component.
    // Replaces %0 with comp.go.
    fn make_guard_dyn_one_cycle_static_comp(
        guard: ir::Guard<ir::StaticTiming>,
        comp_sig: RRC<ir::Cell>,
    ) -> ir::Guard<ir::Nothing> {
        match guard {
            ir::Guard::Or(l, r) => {
                let left = Self::make_guard_dyn_one_cycle_static_comp(
                    *l,
                    Rc::clone(&comp_sig),
                );
                let right = Self::make_guard_dyn_one_cycle_static_comp(
                    *r,
                    Rc::clone(&comp_sig),
                );
                ir::Guard::or(left, right)
            }
            ir::Guard::And(l, r) => {
                let left = Self::make_guard_dyn_one_cycle_static_comp(
                    *l,
                    Rc::clone(&comp_sig),
                );
                let right = Self::make_guard_dyn_one_cycle_static_comp(
                    *r,
                    Rc::clone(&comp_sig),
                );
                ir::Guard::and(left, right)
            }
            ir::Guard::Not(g) => {
                let f = Self::make_guard_dyn_one_cycle_static_comp(
                    *g,
                    Rc::clone(&comp_sig),
                );
                ir::Guard::Not(Box::new(f))
            }
            ir::Guard::Info(t) => match t.get_interval() {
                (0, 1) => guard!(comp_sig["go"]),
                _ => unreachable!(
                    "This function is implemented for 1 cycle static components, only %0 can exist as timing guard"
                ),
            },
            ir::Guard::CompOp(op, l, r) => ir::Guard::CompOp(op, l, r),
            ir::Guard::Port(p) => ir::Guard::Port(p),
            ir::Guard::True => ir::Guard::True,
        }
    }

    // Used for assignments in a one cycle static component.
    // Replaces %0 with comp.go in the assignment's guard.
    fn make_assign_dyn_one_cycle_static_comp(
        assign: ir::Assignment<ir::StaticTiming>,
        comp_sig: RRC<ir::Cell>,
    ) -> ir::Assignment<ir::Nothing> {
        ir::Assignment {
            src: assign.src,
            dst: assign.dst,
            attributes: assign.attributes,
            guard: Box::new(Self::make_guard_dyn_one_cycle_static_comp(
                *assign.guard,
                comp_sig,
            )),
        }
    }

    // Makes `done` signal for promoted static<n> component.
    fn make_done_signal_for_promoted_component(
        fsm_tree: &mut Node,
        builder: &mut ir::Builder,
        comp_sig: RRC<ir::Cell>,
    ) -> Vec<ir::Assignment<ir::Nothing>> {
        let first_state_guard = fsm_tree.query_between((0, 1), builder);
        structure!(builder;
          let sig_reg = prim std_reg(1);
          let one = constant(1, 1);
          let zero = constant(0, 1);
        );
        let go_guard = guard!(comp_sig["go"]);
        let not_go_guard = !guard!(comp_sig["go"]);
        let comp_done_guard =
            first_state_guard.clone().and(guard!(sig_reg["out"]));
        let assigns = build_assignments!(builder;
          // Only write to sig_reg when fsm == 0
          sig_reg["write_en"] = first_state_guard ? one["out"];
          // If fsm == 0 and comp.go is high, it means we are starting an execution,
          // so we set signal_reg to high. Note that this happens regardless of
          // whether comp.done is high.
          sig_reg["in"] = go_guard ? one["out"];
          // Otherwise, we set `sig_reg` to low.
          sig_reg["in"] = not_go_guard ? zero["out"];
          // comp.done is high when FSM == 0 and sig_reg is high,
          // since that means we have just finished an execution.
          comp_sig["done"] = comp_done_guard ? one["out"];
        );
        assigns.to_vec()
    }

    // Makes a done signal for a one-cycle static component.
    // Essentially you just have to use a one-cycle delay register that
    // takes the `go` signal as input.
    fn make_done_signal_for_promoted_component_one_cycle(
        builder: &mut ir::Builder,
        comp_sig: RRC<ir::Cell>,
    ) -> Vec<ir::Assignment<ir::Nothing>> {
        structure!(builder;
          let sig_reg = prim std_reg(1);
          let one = constant(1, 1);
          let zero = constant(0, 1);
        );
        let go_guard = guard!(comp_sig["go"]);
        let not_go = !guard!(comp_sig["go"]);
        let signal_on_guard = guard!(sig_reg["out"]);
        let assigns = build_assignments!(builder;
          // For one cycle components, comp.done is just whatever comp.go
          // was during the previous cycle.
          // signal_reg serves as a forwarding register that delays
          // the `go` signal for one cycle.
          sig_reg["in"] = go_guard ? one["out"];
          sig_reg["in"] = not_go ? zero["out"];
          sig_reg["write_en"] = ? one["out"];
          comp_sig["done"] = signal_on_guard ? one["out"];
        );
        assigns.to_vec()
    }

    // `fsm_tree` should be the top-level tree in the components.
    // `static_groups` are the component's static groups.
    // The assignments are removed from `sgroup` and placed into
    // `builder.component`'s continuous assignments.
    fn compile_static_interface(
        &mut self,
        fsm_tree: &mut Node,
        static_groups: &mut Vec<ir::RRC<ir::StaticGroup>>,
        coloring: &HashMap<ir::Id, ir::Id>,
        colors_to_max_values: &HashMap<ir::Id, (u64, u64)>,
        colors_to_fsm: &mut HashMap<
            ir::Id,
            (OptionalStaticFSM, OptionalStaticFSM),
        >,
        builder: &mut ir::Builder,
    ) -> calyx_utils::CalyxResult<()> {
        if fsm_tree.get_latency() > 1 {
            // Find top-level static group.
            let sgroup = Self::find_static_group(
                &fsm_tree.get_root_name(),
                static_groups,
            );
            // Perform some preprocessing on the assignments
            // (in particular, transform %[0:n] into %0 | %[1:n])
            for assign in &mut sgroup.borrow_mut().assignments {
                Node::preprocess_static_interface_assigns(
                    assign,
                    Rc::clone(&builder.component.signature),
                );
            }

            let comp_go = ir::Guard::port(
                builder
                    .component
                    .signature
                    .borrow()
                    .find_unique_with_attr(ir::NumAttr::Go)?
                    .unwrap(),
            );

            // Realize the fsm tree in hardware.
            fsm_tree.instantiate_fsms(
                builder,
                coloring,
                colors_to_max_values,
                colors_to_fsm,
                self.one_hot_cutoff,
            );
            fsm_tree.count_to_n(builder, Some(comp_go));
            fsm_tree.realize(
                false,
                static_groups,
                &mut self.reset_early_map,
                &mut self.fsm_info_map,
                &mut self.group_rewrites,
                builder,
            );
            // Add root's assignments as continuous assignments, execpt for the
            // `group[done]` assignments.
            builder.component.continuous_assignments.extend(
                fsm_tree.take_root_assigns().into_iter().filter(|assign| {
                    let dst = assign.dst.borrow();
                    match dst.parent {
                        PortParent::Cell(_) => true,
                        // Don't add assignment to `group[done]`
                        PortParent::Group(_) => dst.name != "done",
                        PortParent::StaticGroup(_) => true,
                        PortParent::FSM(_) => unreachable!(),
                    }
                }),
            );
            let comp_sig = Rc::clone(&builder.component.signature);
            if builder.component.attributes.has(ir::BoolAttr::Promoted) {
                // If necessary, add the logic to produce a done signal.
                let done_assigns =
                    Self::make_done_signal_for_promoted_component(
                        fsm_tree, builder, comp_sig,
                    );
                builder
                    .component
                    .continuous_assignments
                    .extend(done_assigns);
            }
        } else {
            // Handle components with latency == 1.
            // In this case, we don't need an FSM; we just guard the assignments
            // with comp.go.
            let sgroup = Self::find_static_group(
                &fsm_tree.get_root_name(),
                static_groups,
            );
            for (child, _) in fsm_tree.get_children() {
                // We can ignore any static timing guards in the children,
                // since we know the latency is 1. That is why we call
                // `convert_assignments_type`.
                child.realize(
                    true,
                    static_groups,
                    &mut self.reset_early_map,
                    &mut self.fsm_info_map,
                    &mut self.group_rewrites,
                    builder,
                )
            }
            let assignments =
                std::mem::take(&mut sgroup.borrow_mut().assignments);
            for mut assign in assignments {
                // Make `assignments` continuous and replace %[0:1] with `comp.go`
                let comp_sig = Rc::clone(&builder.component.signature);
                assign.guard.update(|g| g.and(guard!(comp_sig["go"])));
                builder.component.continuous_assignments.push(
                    Self::make_assign_dyn_one_cycle_static_comp(
                        assign, comp_sig,
                    ),
                );
            }
            if builder.component.attributes.has(ir::BoolAttr::Promoted) {
                // Need to add a done signal if this component was promoted.
                let comp_sig = Rc::clone(&builder.component.signature);
                let done_assigns =
                    Self::make_done_signal_for_promoted_component_one_cycle(
                        builder, comp_sig,
                    );
                builder
                    .component
                    .continuous_assignments
                    .extend(done_assigns);
            };
        };
        Ok(())
    }
}

impl Visitor for CompileStatic {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Drain static groups of component
        let mut sgroups: Vec<ir::RRC<ir::StaticGroup>> =
            comp.get_static_groups_mut().drain().collect();

        let mut builder = ir::Builder::new(comp, sigs);
        // Get a vec of all groups that are enabled in comp's control.
        let static_enable_ids =
            Self::get_static_enables(&builder.component.control.borrow());
        // Build one tree object per static enable.
        // Even if we don't offload, we still need to build trees to
        // determine coloring.
        let default_tree_objects = static_enable_ids
            .iter()
            .map(|id| {
                if self.offload_pause {
                    Self::build_tree_object(*id, &sgroups, 1)
                } else {
                    // If we're not offloading, then we should build dummy trees
                    // that are just used to determine coloring.
                    // This is basically the same thing as `build_tree_object`,
                    // but doesn't check the `ParCtrl` attribute. I think
                    // we could reduce code size by merging this function with
                    // `build_tree_object`.
                    Self::build_dummy_tree(*id, &sgroups)
                }
            })
            .collect_vec();

        // The first thing is to assign FSMs -> static islands.
        // We sometimes assign the same FSM to different static islands
        // to reduce register usage. We do this by getting greedy coloring.
        let coloring: HashMap<ir::Id, ir::Id> = self.get_coloring(
            &default_tree_objects,
            &sgroups,
            &mut builder.component.control.borrow_mut(),
        );
        // We need the max_num_states  and max_num_repeats for each
        // color so we know how many bits the corresponding registers should get.
        let colors_to_max_values =
            Self::get_color_max_values(&coloring, &default_tree_objects);
        let mut colors_to_fsms: HashMap<
            ir::Id,
            (OptionalStaticFSM, OptionalStaticFSM),
        > = HashMap::new();

        let mut tree_objects = if self.offload_pause {
            default_tree_objects
        } else {
            // Build simple trees if we're not offloading (i.e., build trees
            // that just consist of a single node.)
            // Note that these trees would not correctly draw conflicts between
            // nodes for coloring.
            let mut simple_trees = vec![];
            let sgroup_names = sgroups
                .iter()
                .map(|sgroup| sgroup.borrow().name())
                .collect_vec();
            for name in sgroup_names {
                simple_trees.push(Self::build_single_node(name, &sgroups))
            }
            simple_trees
        };

        // Static components have a different interface than static groups.
        // If we have a static component, we have to compile the top-level
        // island (this island should be a group by now and corresponds
        // to the the entire control of the component) differently.
        // This island might invoke other static groups-- these static groups
        // should still follow the group interface.
        let top_level_sgroup = if builder.component.is_static() {
            let comp_control = builder.component.control.borrow();
            match &*comp_control {
                ir::Control::Static(ir::StaticControl::Enable(sen)) => {
                    Some(sen.group.borrow().name())
                }
                _ => {
                    return Err(Error::malformed_control(format!(
                        "Non-Enable Static Control should have been compiled away. Run {} to do this",
                        crate::passes::StaticInliner::name()
                    )));
                }
            }
        } else {
            None
        };
        // Make each tree count to n.
        for tree in &mut tree_objects {
            // Check whether we are compiling the top level static island.
            let static_component_interface = match top_level_sgroup {
                None => false,
                // For the top level group, sch.static_groups should really only
                // have one group--the top level group.
                Some(top_level_group) => {
                    tree.get_group_name() == top_level_group
                }
            };
            // Static component/groups have different interfaces
            if static_component_interface {
                // Compile top level static group differently.
                self.compile_static_interface(
                    tree,
                    &mut sgroups,
                    &coloring,
                    &colors_to_max_values,
                    &mut colors_to_fsms,
                    &mut builder,
                )?;
            } else {
                // Otherwise just instantiate the tree to hardware.
                tree.instantiate_fsms(
                    &mut builder,
                    &coloring,
                    &colors_to_max_values,
                    &mut colors_to_fsms,
                    self.one_hot_cutoff,
                );
                tree.count_to_n(&mut builder, None);
                tree.realize(
                    false,
                    &sgroups,
                    &mut self.reset_early_map,
                    &mut self.fsm_info_map,
                    &mut self.group_rewrites,
                    &mut builder,
                );
            }
        }

        // Rewrite static_group[go] to early_reset_group[go]
        // don't have to worry about writing static_group[done] b/c static
        // groups don't have done holes.
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                self.group_rewrites
                    .get(&port.borrow().canonical())
                    .map(Rc::clone)
            })
        });

        // Add the static groups back to the component.
        comp.get_static_groups_mut().append(sgroups.into_iter());

        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Static] node.
    fn start_static_control(
        &mut self,
        sc: &mut ir::StaticControl,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // The main purpose of this method is inserting wrappers / singal registers
        // when appropriate.

        // No need to build wrapper for static component interface
        if comp.is_static() {
            return Ok(Action::Continue);
        }
        // Assume that there are only static enables left.
        // If there are any other type of static control, then error out.
        let ir::StaticControl::Enable(s) = sc else {
            return Err(Error::malformed_control(format!(
                "Non-Enable Static Control should have been compiled away. Run {} to do this",
                crate::passes::StaticInliner::name()
            )));
        };

        let sgroup = s.group.borrow_mut();
        let sgroup_name = sgroup.name();
        // get the "early reset group". It should exist, since we made an
        // early_reset group for every static group in the component
        let early_reset_name =
            self.reset_early_map.get(&sgroup_name).unwrap_or_else(|| {
                unreachable!(
                    "{}'s early reset group has not been created",
                    sgroup_name
                )
            });
        // check if we've already built the wrapper group for early_reset_group
        // if so, we can just use that, otherwise, we must build the wrapper group
        let group_choice = match self.wrapper_map.get(early_reset_name) {
            None => {
                // create the builder/cells that we need to create wrapper group
                let mut builder = ir::Builder::new(comp, sigs);
                let (fsm_name, _, fsm_final_state) = self.fsm_info_map.get(early_reset_name).unwrap_or_else(|| unreachable!("group {} has no correspondoing fsm in self.fsm_map", early_reset_name));
                // If we've already made a wrapper for a group that uses the same
                // FSM, we can reuse the signal_reg. Otherwise, we must
                // instantiate a new signal_reg.
                let wrapper = match self.signal_reg_map.get(fsm_name) {
                    None => {
                        // Need to build the signal_reg and the continuous
                        // assignment that resets the signal_reg
                        structure!( builder;
                            let signal_reg = prim std_reg(1);
                        );
                        self.signal_reg_map
                            .insert(*fsm_name, signal_reg.borrow().name());
                        Self::build_wrapper_group(
                            fsm_final_state.clone(),
                            early_reset_name,
                            signal_reg,
                            &mut builder,
                            true,
                        )
                    }
                    Some(reg_name) => {
                        // Already_built the signal_reg.
                        // We don't need to add continuous assignments
                        // that resets signal_reg.
                        let signal_reg = builder
                            .component
                            .find_cell(*reg_name)
                            .unwrap_or_else(|| {
                                unreachable!("signal reg {reg_name} found")
                            });
                        Self::build_wrapper_group(
                            fsm_final_state.clone(),
                            early_reset_name,
                            signal_reg,
                            &mut builder,
                            false,
                        )
                    }
                };
                self.wrapper_map
                    .insert(*early_reset_name, wrapper.borrow().name());
                wrapper
            }
            Some(name) => comp.find_group(*name).unwrap(),
        };

        let mut e = ir::Control::enable(group_choice);
        let attrs = std::mem::take(&mut s.attributes);
        *e.get_mut_attributes() = attrs;
        Ok(Action::Change(Box::new(e)))
    }

    /// If while body is static, then we want to make sure that the while
    /// body does not take the extra cycle incurred by the done condition
    /// So we replace the while loop with `enable` of a wrapper group
    /// that sets the go signal of the static group in the while loop body high
    /// (all static control should be compiled into static groups by
    /// `static_inliner` now). The done signal of the wrapper group should be
    /// the condition that the fsm of the while body is %0 and the port signal
    /// is 1'd0.
    /// For example, we replace
    /// ```
    /// wires {
    /// static group A<1> {
    ///     ...
    ///   }
    ///    ...
    /// }
    /// control {
    ///   while l.out {
    ///     A;
    ///   }
    /// }
    /// ```
    /// with
    /// ```
    /// wires {
    ///  group early_reset_A {
    ///     ...
    ///        }
    ///
    /// group while_wrapper_early_reset_A {
    ///       early_reset_A[go] = 1'd1;
    ///       while_wrapper_early_reset_A[done] = !l.out & fsm.out == 1'd0 ? 1'd1;
    ///     }
    ///   }
    ///   control {
    ///     while_wrapper_early_reset_A;
    ///   }
    /// ```
    fn start_while(
        &mut self,
        s: &mut ir::While,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.cond.is_none() {
            if let ir::Control::Static(sc) = &mut *(s.body) {
                let mut builder = ir::Builder::new(comp, sigs);
                let reset_group_name = self.get_reset_group_name(sc);

                // Get fsm for reset_group
                let (_, fsm_first_state, _) = self.fsm_info_map.get(reset_group_name).unwrap_or_else(|| unreachable!("group {} has no correspondoing fsm in self.fsm_map", reset_group_name));
                let wrapper_group = self.build_wrapper_group_while(
                    fsm_first_state.clone(),
                    reset_group_name,
                    Rc::clone(&s.port),
                    &mut builder,
                );
                let c = ir::Control::enable(wrapper_group);
                return Ok(Action::change(c));
            }
        }

        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // make sure static groups have no assignments, since
        // we should have already drained the assignments in static groups
        // for g in comp.get_static_groups() {
        //     if !g.borrow().assignments.is_empty() {
        //         unreachable!("Should have converted all static groups to dynamic. {} still has assignments in it. It's possible that you may need to run {} to remove dead groups and get rid of this error.", g.borrow().name(), crate::passes::DeadGroupRemoval::name());
        //     }
        // }
        // remove all static groups
        comp.get_static_groups_mut().retain(|_| false);

        // Remove control if static component
        if comp.is_static() {
            comp.control = ir::rrc(ir::Control::empty())
        }

        Ok(Action::Continue)
    }
}
