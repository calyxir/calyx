use super::math_utilities::get_bit_width_from;
use crate::analysis::GraphColoring;
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use calyx_ir::{guard, structure, GetAttributes};
use calyx_utils::Error;
use ir::{build_assignments, Nothing, StaticTiming, RRC};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::ops::Not;
use std::rc::Rc;

#[derive(Default)]
/// Compiles Static Islands
pub struct CompileStatic {
    /// maps original static group names to the corresponding group that has an FSM that reset early
    reset_early_map: HashMap<ir::Id, ir::Id>,
    /// maps group that has an FSM that resets early to its dynamic "wrapper" group name.
    wrapper_map: HashMap<ir::Id, ir::Id>,
    /// maps fsm names to their corresponding signal_reg
    signal_reg_map: HashMap<ir::Id, ir::Id>,
    /// maps reset_early_group names to (fsm name, fsm_width)
    fsm_info_map: HashMap<ir::Id, (ir::Id, u64)>,
    /// rewrites `static_group[go]` to `dynamic_group[go]`
    group_rewrite: ir::rewriter::PortRewriteMap,
}

impl Named for CompileStatic {
    fn name() -> &'static str {
        "compile-static"
    }

    fn description() -> &'static str {
        "Compiles Static Islands"
    }
}

// Takes in a static guard `guard`, and returns equivalent dynamic guard
// The only thing that actually changes is the Guard::Info case
// We need to turn static_timing to dynamic guards using `fsm`.
// E.g.: %[2:3] gets turned into fsm.out >= 2 & fsm.out < 3
fn make_guard_dyn(
    guard: ir::Guard<StaticTiming>,
    fsm: &ir::RRC<ir::Cell>,
    fsm_size: u64,
    builder: &mut ir::Builder,
) -> Box<ir::Guard<Nothing>> {
    match guard {
        ir::Guard::Or(l, r) => Box::new(ir::Guard::Or(
            make_guard_dyn(*l, fsm, fsm_size, builder),
            make_guard_dyn(*r, fsm, fsm_size, builder),
        )),
        ir::Guard::And(l, r) => Box::new(ir::Guard::And(
            make_guard_dyn(*l, fsm, fsm_size, builder),
            make_guard_dyn(*r, fsm, fsm_size, builder),
        )),
        ir::Guard::Not(g) => {
            Box::new(ir::Guard::Not(make_guard_dyn(*g, fsm, fsm_size, builder)))
        }
        ir::Guard::CompOp(op, l, r) => Box::new(ir::Guard::CompOp(op, l, r)),
        ir::Guard::Port(p) => Box::new(ir::Guard::Port(p)),
        ir::Guard::True => Box::new(ir::Guard::True),
        ir::Guard::Info(static_timing) => {
            let (beg, end) = static_timing.get_interval();
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
fn make_assign_dyn(
    assign: ir::Assignment<StaticTiming>,
    fsm: &ir::RRC<ir::Cell>,
    fsm_size: u64,
    builder: &mut ir::Builder,
) -> ir::Assignment<Nothing> {
    ir::Assignment {
        src: assign.src,
        dst: assign.dst,
        attributes: assign.attributes,
        guard: make_guard_dyn(*assign.guard, fsm, fsm_size, builder),
    }
}

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

// Given an input static_group `sgroup`, finds the names of all of the groups
// that it triggers through their go hole.
// E.g., if `sgroup` has assignments that write to `sgroup1[go]` and `sgroup2[go]`
// then return `{sgroup1, sgroup2}`
// NOTE: assumes that static groups will only write the go holes of other static
// groups, and never dynamic groups
fn get_go_writes(sgroup: &ir::RRC<ir::StaticGroup>) -> HashSet<ir::Id> {
    let mut uses = HashSet::new();
    for asgn in &sgroup.borrow().assignments {
        let dst = asgn.dst.borrow();
        if dst.is_hole() && dst.name == "go" {
            uses.insert(dst.get_parent_name());
        }
    }
    uses
}

impl CompileStatic {
    // returns an "early reset" group based on the information given
    // in the arguments.
    // sgroup_assigns are the static assignments of the group (they need to be
    // changed to dynamic by instantiating an fsm, i.e., %[0,2] -> fsm.out < 2)
    // name of early reset group has prefix "early_reset_{sgroup_name}"
    fn make_early_reset_group(
        &mut self,
        sgroup_assigns: &mut Vec<ir::Assignment<ir::StaticTiming>>,
        sgroup_name: ir::Id,
        latency: u64,
        attributes: ir::Attributes,
        fsm: ir::RRC<ir::Cell>,
        builder: &mut ir::Builder,
    ) -> ir::RRC<ir::Group> {
        let fsm_name = fsm.borrow().name();
        let fsm_size = fsm
            .borrow()
            .find("out")
            .unwrap_or_else(|| unreachable!("no `out` port on {fsm_name}"))
            .borrow()
            .width;
        structure!( builder;
            // done hole will be undefined bc of early reset
            let ud = prim undef(1);
            let signal_on = constant(1,1);
            let adder = prim std_add(fsm_size);
            let const_one = constant(1, fsm_size);
            let first_state = constant(0, fsm_size);
            let penultimate_state = constant(latency-1, fsm_size);
        );
        // create the dynamic group we will use to replace the static group
        let mut early_reset_name = sgroup_name.to_string();
        early_reset_name.insert_str(0, "early_reset_");
        let g = builder.add_group(early_reset_name);
        // converting static assignments to dynamic assignments
        let mut assigns = sgroup_assigns
            .drain(..)
            .map(|assign| make_assign_dyn(assign, &fsm, fsm_size, builder))
            .collect_vec();
        // assignments to increment the fsm
        let not_penultimate_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"] != penultimate_state["out"]);
        let penultimate_state_guard: ir::Guard<ir::Nothing> =
            guard!(fsm["out"] == penultimate_state["out"]);
        let fsm_incr_assigns = build_assignments!(
          builder;
          // increments the fsm
          adder["left"] = ? fsm["out"];
          adder["right"] = ? const_one["out"];
          fsm["write_en"] = ? signal_on["out"];
          fsm["in"] = not_penultimate_state_guard ? adder["out"];
           // resets the fsm early
          fsm["in"] = penultimate_state_guard ? first_state["out"];
          // will never reach this guard since we are resetting when we get to
          // the penultimate state
          g["done"] = ? ud["out"];
        );
        assigns.extend(fsm_incr_assigns.to_vec());
        // maps the "early reset" group name to the "fsm name" that it borrows.
        // this is helpful when we build the "wrapper group"
        self.fsm_info_map
            .insert(g.borrow().name(), (fsm.borrow().name(), fsm_size));
        // adding the assignments to the new dynamic group and creating a
        // new (dynamic) enable
        g.borrow_mut().assignments = assigns;
        g.borrow_mut().attributes = attributes;
        g
    }

    fn build_wrapper_group(
        fsm_name: &ir::Id,
        fsm_width: u64,
        group_name: &ir::Id,
        signal_reg: ir::RRC<ir::Cell>,
        builder: &mut ir::Builder,
        add_continuous_assigns: bool,
    ) -> ir::RRC<ir::Group> {
        // get the groups/fsm necessary to build the wrapper group
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
        let early_reset_fsm =
            builder.component.find_cell(*fsm_name).unwrap_or_else(|| {
                unreachable!(
                    "called build_wrapper_group with {}, which is not an fsm",
                    fsm_name
                )
            });

        structure!( builder;
            let state_zero = constant(0, fsm_width);
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
        );
        // make guards
        // fsm.out == 0 ?
        let first_state: ir::Guard<ir::Nothing> =
            guard!(early_reset_fsm["out"] == state_zero["out"]);
        // signal_reg.out ?
        let signal_reg_guard: ir::Guard<ir::Nothing> =
            guard!(signal_reg["out"]);
        // !signal_reg.out ?
        let not_signal_reg = signal_reg_guard.clone().not();
        // fsm.out == 0 & signal_reg.out ?
        let first_state_and_signal = first_state.clone() & signal_reg_guard;
        // fsm.out == 0 & ! signal_reg.out ?
        let first_state_and_not_signal = first_state & not_signal_reg;
        // create the wrapper group for early_reset_group
        let mut wrapper_name = group_name.clone().to_string();
        wrapper_name.insert_str(0, "wrapper_");
        let g = builder.add_group(wrapper_name);
        let group_assigns = build_assignments!(
          builder;
          // early_reset_group[go] = 1'd1
          early_reset_group["go"] = ? signal_on["out"];
          // when fsm == 0, and !signal_reg, then set signal_reg to high
          signal_reg["write_en"] = first_state_and_not_signal ? signal_on["out"];
          signal_reg["in"] =  first_state_and_not_signal ? signal_on["out"];
          // group[done] = fsm.out == 0 & signal_reg.out ? 1'd1
          g["done"] = first_state_and_signal ? signal_on["out"];
        );
        if add_continuous_assigns {
            // continuous assignments to reset signal_reg back to 0 when the wrapper is done
            let continuous_assigns = build_assignments!(
                builder;
                // when (fsm == 0 & signal_reg is high), which is the done condition of the wrapper,
                // reset the signal_reg back to low
                signal_reg["write_en"] = first_state_and_signal ? signal_on["out"];
                signal_reg["in"] =  first_state_and_signal ? signal_off["out"];
            );
            builder.add_continuous_assignments(continuous_assigns.to_vec());
        }
        g.borrow_mut().assignments = group_assigns.to_vec();
        g.borrow_mut().attributes =
            early_reset_group.borrow().attributes.clone();
        g
    }

    fn get_reset_group_name(&self, sc: &mut ir::StaticControl) -> &ir::Id {
        // assume that there are only static enables left.
        // if there are any other type of static control, then error out.
        let ir::StaticControl::Enable(s) = sc else {
            unreachable!("Non-Enable Static Control should have been compiled away. Run {} to do this", crate::passes::StaticInliner::name());
        };

        let sgroup = s.group.borrow_mut();
        let sgroup_name = sgroup.name();
        // get the "early reset group". It should exist, since we made an
        // early_reset group for every static group in the component
        let early_reset_name =
            self.reset_early_map.get(&sgroup_name).unwrap_or_else(|| {
                unreachable!(
                    "group {} not in self.reset_early_map",
                    sgroup_name
                )
            });

        early_reset_name
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
        fsm_name: &ir::Id,
        fsm_width: u64,
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
        let early_reset_fsm =
            builder.component.find_cell(*fsm_name).unwrap_or_else(|| {
                unreachable!(
                    "called build_wrapper_group with {}, which is not an fsm",
                    fsm_name
                )
            });

        let wrapper_group =
            builder.add_group(format!("while_wrapper_{}", group_name));

        structure!(
            builder;
            let one = constant(1, 1);
            let time_0 = constant(0, fsm_width);
        );

        let port_parent = port.borrow().cell_parent();
        let port_name = port.borrow().name;
        let done_guard = guard!(port_parent[port_name]).not()
            & guard!(early_reset_fsm["out"] == time_0["out"]);

        let assignments = build_assignments!(
            builder;
            // reset_early_group[go] = 1'd1;
            // wrapper_group[done] = !port ? 1'd1;
            reset_early_group["go"] = ? one["out"];
            wrapper_group["done"] = done_guard ? one["out"];
        );

        wrapper_group.borrow_mut().assignments.extend(assignments);
        wrapper_group
    }

    // Gets all of the triggered static groups within `c`, and adds it to `cur_names`.
    // Relies on sgroup_uses_map to take into account groups that are triggered through
    // their `go` hole.
    fn get_used_sgroups(
        c: &ir::Control,
        cur_names: &mut HashSet<ir::Id>,
        sgroup_uses_map: &HashMap<ir::Id, HashSet<ir::Id>>,
    ) {
        match c {
            ir::Control::Empty(_)
            | ir::Control::Enable(_)
            | ir::Control::Invoke(_) => (),
            ir::Control::Static(sc) => {
                let ir::StaticControl::Enable(s) = sc else {
                    unreachable!("Non-Enable Static Control should have been compiled away. Run {} to do this", crate::passes::StaticInliner::name());
                };
                let group_name = s.group.borrow().name();
                if let Some(sgroup_uses) = sgroup_uses_map.get(&group_name) {
                    cur_names.extend(sgroup_uses);
                }
                cur_names.insert(group_name);
            }
            ir::Control::Par(ir::Par { stmts, .. })
            | ir::Control::Seq(ir::Seq { stmts, .. }) => {
                for stmt in stmts {
                    Self::get_used_sgroups(stmt, cur_names, sgroup_uses_map);
                }
            }
            ir::Control::Repeat(ir::Repeat { body, .. })
            | ir::Control::While(ir::While { body, .. }) => {
                Self::get_used_sgroups(body, cur_names, sgroup_uses_map);
            }
            ir::Control::If(if_stmt) => {
                Self::get_used_sgroups(
                    &if_stmt.tbranch,
                    cur_names,
                    sgroup_uses_map,
                );
                Self::get_used_sgroups(
                    &if_stmt.fbranch,
                    cur_names,
                    sgroup_uses_map,
                );
            }
        }
    }

    /// Given control `c`, adds conflicts to `conflict_graph` between all
    /// static groups that are executed in separate threads of the same par block.
    /// `sgroup_uses_map` maps:
    /// static group names -> all of the static groups that it triggers the go ports
    /// of (even recursively).
    /// Example: group A {B[go] = 1;} group B {C[go] = 1} group C{}
    /// Would map: A -> {B,C} and B -> {C}
    fn add_par_conflicts(
        c: &ir::Control,
        sgroup_uses_map: &HashMap<ir::Id, HashSet<ir::Id>>,
        conflict_graph: &mut GraphColoring<ir::Id>,
    ) {
        match c {
            ir::Control::Empty(_)
            | ir::Control::Enable(_)
            | ir::Control::Invoke(_)
            | ir::Control::Static(_) => (),
            ir::Control::Seq(seq) => {
                for stmt in &seq.stmts {
                    Self::add_par_conflicts(
                        stmt,
                        sgroup_uses_map,
                        conflict_graph,
                    );
                }
            }
            ir::Control::Repeat(ir::Repeat { body, .. })
            | ir::Control::While(ir::While { body, .. }) => {
                Self::add_par_conflicts(body, sgroup_uses_map, conflict_graph)
            }
            ir::Control::If(if_stmt) => {
                Self::add_par_conflicts(
                    &if_stmt.tbranch,
                    sgroup_uses_map,
                    conflict_graph,
                );
                Self::add_par_conflicts(
                    &if_stmt.fbranch,
                    sgroup_uses_map,
                    conflict_graph,
                );
            }
            ir::Control::Par(par) => {
                // sgroup_conflict_vec is a vec of HashSets.
                // Each entry of the vec corresponds to a par thread, and holds
                // all of the groups executed in that thread.
                let mut sgroup_conflict_vec = Vec::new();
                for stmt in &par.stmts {
                    let mut used_sgroups = HashSet::new();
                    Self::get_used_sgroups(
                        stmt,
                        &mut used_sgroups,
                        sgroup_uses_map,
                    );
                    sgroup_conflict_vec.push(used_sgroups);
                }
                for (thread1_sgroups, thread2_sgroups) in
                    sgroup_conflict_vec.iter().tuple_combinations()
                {
                    for sgroup1 in thread1_sgroups {
                        for sgroup2 in thread2_sgroups {
                            conflict_graph.insert_conflict(sgroup1, sgroup2);
                        }
                    }
                }
                // Necessary to add conflicts between nested pars
                for stmt in &par.stmts {
                    Self::add_par_conflicts(
                        stmt,
                        sgroup_uses_map,
                        conflict_graph,
                    );
                }
            }
        }
    }

    /// Given an `sgroup_uses_map`, which maps:
    /// static group names -> all of the static groups that it triggers the go ports
    /// of (even recursively).
    /// Example: group A {B[go] = 1;} group B {C[go] = 1} group C{}
    /// Would map: A -> {B,C} and B -> {C}
    /// Adds conflicts between any groups triggered at the same time based on
    /// `go` port triggering.
    fn add_go_port_conflicts(
        sgroup_uses_map: &HashMap<ir::Id, HashSet<ir::Id>>,
        conflict_graph: &mut GraphColoring<ir::Id>,
    ) {
        for (sgroup, sgroup_uses) in sgroup_uses_map {
            for sgroup_use in sgroup_uses {
                conflict_graph.insert_conflict(sgroup_use, sgroup);
            }
            // If multiple groups are triggered by the same group, then
            // we conservatively add a conflict between such groups
            for (sgroup_use1, sgroup_use2) in
                sgroup_uses.iter().tuple_combinations()
            {
                conflict_graph.insert_conflict(sgroup_use1, sgroup_use2);
            }
        }
    }

    // Given a "coloring" of static group names -> their "colors",
    // instantiate one fsm per color and return a hashmap that maps
    // fsm names -> groups that it handles
    fn build_fsm_mapping(
        coloring: HashMap<ir::Id, ir::Id>,
        static_groups: &[ir::RRC<ir::StaticGroup>],
        builder: &mut ir::Builder,
    ) -> HashMap<ir::Id, HashSet<ir::Id>> {
        // "reverse" the coloring to map colors -> static group_names
        let mut color_to_groups: HashMap<ir::Id, HashSet<ir::Id>> =
            HashMap::new();
        for (group, color) in coloring {
            color_to_groups.entry(color).or_default().insert(group);
        }
        // Need deterministic ordering for testing.
        let mut vec_color_to_groups: Vec<(ir::Id, HashSet<ir::Id>)> =
            color_to_groups.into_iter().collect();
        vec_color_to_groups
            .sort_by(|(color1, _), (color2, _)| color1.cmp(color2));
        vec_color_to_groups.into_iter().map(|(color, group_names)| {
            // For each color, build an FSM that has the number of bits required
            // for the largest latency in `group_names`
            let max_latency = group_names
                .iter()
                .map(|g| {
                    find_static_group(g, static_groups).borrow()
                        .latency
                })
                .max().unwrap_or_else(|| unreachable!("group {color} had no corresponding groups in its coloring map")
                );
            let fsm_size = get_bit_width_from(
                max_latency + 1, /* represent 0..latency */
            );
            structure!( builder;
                let fsm = prim std_reg(fsm_size);
            );
            let fsm_name = fsm.borrow().name();
            (fsm_name, group_names)
        }).collect()
    }

    // helper to `build_sgroup_uses_map`
    // `parent_group` is the group that we are "currently" analyzing
    // `full_group_ancestry` is the "ancestry of the group we are analyzing"
    // Example: group A {B[go] = 1;} group B {C[go] = 1} group C{}, and `parent_group`
    // is B, then ancestry would be B and A.
    // `cur_mapping` is the current_mapping for `sgroup_uses_map`
    // `group_names` is a vec of group_names. Once we analyze a group, we should
    // remove it from group_names
    // `sgroups` is a vec of static groups.
    fn update_sgroup_uses_map(
        parent_group: &ir::Id,
        full_group_ancestry: &mut HashSet<ir::Id>,
        cur_mapping: &mut HashMap<ir::Id, HashSet<ir::Id>>,
        group_names: &mut HashSet<ir::Id>,
        sgroups: &Vec<ir::RRC<ir::StaticGroup>>,
    ) {
        let group_uses =
            get_go_writes(&find_static_group(parent_group, sgroups));
        for group_use in group_uses {
            for ancestor in full_group_ancestry.iter() {
                cur_mapping.entry(*ancestor).or_default().insert(group_use);
            }
            full_group_ancestry.insert(group_use);
            Self::update_sgroup_uses_map(
                &group_use,
                full_group_ancestry,
                cur_mapping,
                group_names,
                sgroups,
            );
            full_group_ancestry.remove(&group_use);
        }
        group_names.remove(parent_group);
    }

    /// Builds an `sgroup_uses_map`, which maps:
    /// static group names -> all of the static groups that it triggers the go ports
    /// of (even recursively).
    /// Example: group A {B[go] = 1;} group B {C[go] = 1} group C{}
    /// Would map: A -> {B,C} and B -> {C}
    fn build_sgroup_uses_map(
        sgroups: &Vec<ir::RRC<ir::StaticGroup>>,
    ) -> HashMap<ir::Id, HashSet<ir::Id>> {
        let mut names: HashSet<ir::Id> = sgroups
            .iter()
            .map(|sgroup| sgroup.borrow().name())
            .collect();
        let mut cur_mapping = HashMap::new();
        while !names.is_empty() {
            let random_group = *names.iter().next().unwrap();
            Self::update_sgroup_uses_map(
                &random_group,
                &mut HashSet::from([random_group]),
                &mut cur_mapping,
                &mut names,
                sgroups,
            )
        }
        cur_mapping
    }
}

impl Visitor for CompileStatic {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let sgroups: Vec<ir::RRC<ir::StaticGroup>> =
            comp.get_static_groups_mut().drain().collect();
        // `sgroup_uses_map` builds a mapping of static groups -> groups that
        // it (even indirectly) triggers the `go` port of.
        let sgroup_uses_map = Self::build_sgroup_uses_map(&sgroups);
        // Build conflict graph and get coloring.
        let mut conflict_graph: GraphColoring<ir::Id> =
            GraphColoring::from(sgroups.iter().map(|g| g.borrow().name()));
        Self::add_par_conflicts(
            &comp.control.borrow(),
            &sgroup_uses_map,
            &mut conflict_graph,
        );
        Self::add_go_port_conflicts(&sgroup_uses_map, &mut conflict_graph);
        let coloring = conflict_graph.color_greedy(None, true);
        let mut builder = ir::Builder::new(comp, sigs);
        // build Mappings of fsm names -> set of groups that it can handle.
        let fsm_mappings =
            Self::build_fsm_mapping(coloring, &sgroups, &mut builder);
        let mut groups_to_fsms = HashMap::new();
        // "Reverses" fsm_mappings to map group names -> fsm cells
        for (fsm_name, group_names) in fsm_mappings {
            let fsm = builder.component.find_guaranteed_cell(fsm_name);
            for group_name in group_names {
                groups_to_fsms.insert(group_name, Rc::clone(&fsm));
            }
        }

        // create "early reset" dynamic groups that never reach set their done hole
        for sgroup in sgroups.iter() {
            let mut sgroup_ref = sgroup.borrow_mut();
            let sgroup_name = sgroup_ref.name();
            let sgroup_latency = sgroup_ref.get_latency();
            let sgroup_attributes = sgroup_ref.attributes.clone();
            let sgroup_assigns = &mut sgroup_ref.assignments;
            let g = self.make_early_reset_group(
                sgroup_assigns,
                sgroup_name,
                sgroup_latency,
                sgroup_attributes,
                Rc::clone(groups_to_fsms.get(&sgroup_name).unwrap_or_else(
                    || unreachable!("{sgroup_name} has no corresponding fsm"),
                )),
                &mut builder,
            );
            // map the static group name -> early reset group name
            // helpful for rewriting control
            self.reset_early_map.insert(sgroup_name, g.borrow().name());
            // group_rewrite_map helps write static_group[go] to early_reset_group[go]
            // technically could do this w/ early_reset_map but is easier w/
            // group_rewrite, which is explicitly of type `PortRewriterMap`
            self.group_rewrite.insert(
                ir::Canonical(sgroup_name, ir::Id::from("go")),
                g.borrow().find("go").unwrap_or_else(|| {
                    unreachable!("group {} has no go port", g.borrow().name())
                }),
            );
        }

        // rewrite static_group[go] to early_reset_group[go]
        // don't have to worry about writing static_group[done] b/c static
        // groups don't have done holes.
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                self.group_rewrite
                    .get(&port.borrow().canonical())
                    .map(Rc::clone)
            })
        });

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
        // assume that there are only static enables left.
        // if there are any other type of static control, then error out.
        let ir::StaticControl::Enable(s) = sc else {
            return Err(Error::malformed_control(format!("Non-Enable Static Control should have been compiled away. Run {} to do this", crate::passes::StaticInliner::name())));
        };

        let sgroup = s.group.borrow_mut();
        let sgroup_name = sgroup.name();
        // get the "early reset group". It should exist, since we made an
        // early_reset group for every static group in the component
        let early_reset_name =
            self.reset_early_map.get(&sgroup_name).unwrap_or_else(|| {
                unreachable!(
                    "group {} early reset has not been created",
                    sgroup_name
                )
            });
        // check if we've already built the wrapper group for early_reset_group
        // if so, we can just use that, otherwise, we must build the wrapper group
        let group_choice = match self.wrapper_map.get(early_reset_name) {
            None => {
                // create the builder/cells that we need to create wrapper group
                let mut builder = ir::Builder::new(comp, sigs);
                let (fsm_name, fsm_width )= self.fsm_info_map.get(early_reset_name).unwrap_or_else(|| unreachable!("group {} has no correspondoing fsm in self.fsm_map", early_reset_name));
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
                            fsm_name,
                            *fsm_width,
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
                            fsm_name,
                            *fsm_width,
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

    /// if while body is static, then we want to make sure that the while
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
                let (fsm, fsm_width) = self.fsm_info_map.get(reset_group_name).unwrap_or_else(|| unreachable!("group {} has no correspondoing fsm in self.fsm_map", reset_group_name));
                let wrapper_group = self.build_wrapper_group_while(
                    fsm,
                    *fsm_width,
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
        for g in comp.get_static_groups() {
            if !g.borrow().assignments.is_empty() {
                unreachable!("Should have converted all static groups to dynamic. {} still has assignments in it", g.borrow().name());
            }
        }
        // remove all static groups
        comp.get_static_groups_mut().retain(|_| false);
        Ok(Action::Continue)
    }
}
