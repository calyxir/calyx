use crate::analysis::{GraphColoring, StaticFSM, StaticSchedule};
use crate::traversal::{Action, Named, ParseVal, PassOpt, VisResult, Visitor};
use calyx_ir as ir;
use calyx_ir::{guard, structure, GetAttributes};
use calyx_utils::Error;
use ir::{build_assignments, RRC};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::ops::Not;
use std::rc::Rc;

/// Compiles Static Islands
#[derive(Default)]
pub struct CompileStatic {
    /// maps original static group names to the corresponding group that has an FSM that reset early
    reset_early_map: HashMap<ir::Id, ir::Id>,
    /// maps group that has an FSM that resets early to its dynamic "wrapper" group name.
    wrapper_map: HashMap<ir::Id, ir::Id>,
    /// maps fsm names to their corresponding signal_reg
    signal_reg_map: HashMap<ir::Id, ir::Id>,
    /// maps reset_early_group names to StaticFSM object
    fsm_info_map: HashMap<ir::Id, ir::RRC<StaticFSM>>,
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
            "The upper limit on the number of states the static FSM must have before we pick binary \
            encoding over one-hot. Defaults to 0 (i.e., always choose binary encoding)",
            ParseVal::Num(0),
            PassOpt::parse_num,
        )]
    }
}

impl CompileStatic {
    /// Builds a wrapper group for group named group_name using fsm and
    /// and a signal_reg.
    /// Both the group and FSM (and the signal_reg) should already exist.
    /// `add_resetting_logic` is a bool; since the same FSM/signal_reg pairing
    /// may be used for multiple static islands, and we only add resetting logic
    /// for the signal_reg once.
    fn build_wrapper_group(
        fsm_object: ir::RRC<StaticFSM>,
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

        // fsm.out == 0
        let first_state =
            *fsm_object.borrow_mut().query_between(builder, (0, 1));
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
        // fsm.out == 0 & signal_reg.out
        let first_state_and_signal = first_state.clone() & signal_reg_guard;
        // fsm.out == 0 & ! signal_reg.out
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
        if add_reseting_logic {
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
        fsm_object: ir::RRC<StaticFSM>,
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

        let fsm_eq_0 = *fsm_object.borrow_mut().query_between(builder, (0, 1));

        let wrapper_group =
            builder.add_group(format!("while_wrapper_{}", group_name));

        structure!(
            builder;
            let one = constant(1, 1);
        );

        let port_parent = port.borrow().cell_parent();
        let port_name = port.borrow().name;
        let done_guard = guard!(port_parent[port_name]).not() & fsm_eq_0;

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

    // Given a `coloring` of static group names, along with the actual `static_groups`,
    // it builds one StaticSchedule per color.
    fn build_schedule_objects(
        coloring: HashMap<ir::Id, ir::Id>,
        mut static_groups: Vec<ir::RRC<ir::StaticGroup>>,
        _builder: &mut ir::Builder,
    ) -> Vec<StaticSchedule> {
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
        vec_color_to_groups
            .into_iter()
            .map(|(_, group_names)| {
                // For each color, build a StaticSchedule object.
                // We first have to figure out out which groups we need to
                // build the static_schedule object for.
                let (matching_groups, other_groups) =
                    static_groups.drain(..).partition(|group| {
                        group_names.contains(&group.borrow().name())
                    });
                let sch = StaticSchedule::from(matching_groups);
                static_groups = other_groups;
                sch
            })
            .collect()
    }

    // Get early reset group name from static control (we assume the static control
    // is an enable).
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
}

// These are the functions used to allocate FSMs to static islands
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

    // Given an input static_group `sgroup`, finds the names of all of the groups
    // that it triggers through their go hole.
    // E.g., if `sgroup` has assignments that write to `sgroup1[go]` and `sgroup2[go]`
    // then return `{sgroup1, sgroup2}`
    // Assumes that static groups will only write the go holes of other static
    // groups, and never dynamic groups (which seems like a reasonable assumption).
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

    /// Adds conflicts between static groups that require different encodings.
    /// For example: if one group is one-hot and another is binary, then
    /// we insert a conflict between those two groups.
    fn add_encoding_conflicts(
        sgroups: &[ir::RRC<ir::StaticGroup>],
        conflict_graph: &mut GraphColoring<ir::Id>,
    ) {
        for (sgroup1, sgroup2) in sgroups.iter().tuple_combinations() {
            if sgroup1.borrow().attributes.has(ir::BoolAttr::OneHot)
                != sgroup2.borrow().attributes.has(ir::BoolAttr::OneHot)
            {
                conflict_graph.insert_conflict(
                    &sgroup1.borrow().name(),
                    &sgroup2.borrow().name(),
                );
            }
        }
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
        let group_uses = Self::get_go_writes(&Self::find_static_group(
            parent_group,
            sgroups,
        ));
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

    pub fn get_coloring(
        sgroups: &Vec<ir::RRC<ir::StaticGroup>>,
        control: &ir::Control,
    ) -> HashMap<ir::Id, ir::Id> {
        // `sgroup_uses_map` builds a mapping of static groups -> groups that
        // it (even indirectly) triggers the `go` port of.
        let sgroup_uses_map = Self::build_sgroup_uses_map(sgroups);
        // Build conflict graph and get coloring.
        let mut conflict_graph: GraphColoring<ir::Id> =
            GraphColoring::from(sgroups.iter().map(|g| g.borrow().name()));
        Self::add_par_conflicts(control, &sgroup_uses_map, &mut conflict_graph);
        Self::add_go_port_conflicts(&sgroup_uses_map, &mut conflict_graph);
        Self::add_encoding_conflicts(sgroups, &mut conflict_graph);
        conflict_graph.color_greedy(None, true)
    }
}

// These are the functions used to compile for the static *component* interface
impl CompileStatic {
    // Used for guards in a one cycle static component.
    // Replaces %0 with comp.go.
    fn make_guard_dyn_one_cycle_static_comp(
        guard: ir::Guard<ir::StaticTiming>,
        comp_sig: RRC<ir::Cell>,
    ) -> ir::Guard<ir::Nothing> {
        match guard {
        ir::Guard::Or(l, r) => {
            let left =
                Self::make_guard_dyn_one_cycle_static_comp(*l, Rc::clone(&comp_sig));
            let right = Self::make_guard_dyn_one_cycle_static_comp(*r, Rc::clone(&comp_sig));
            ir::Guard::or(left, right)
        }
        ir::Guard::And(l, r) => {
            let left = Self::make_guard_dyn_one_cycle_static_comp(*l, Rc::clone(&comp_sig));
            let right = Self::make_guard_dyn_one_cycle_static_comp(*r, Rc::clone(&comp_sig));
            ir::Guard::and(left, right)
        }
        ir::Guard::Not(g) => {
            let f = Self::make_guard_dyn_one_cycle_static_comp(*g, Rc::clone(&comp_sig));
            ir::Guard::Not(Box::new(f))
        }
        ir::Guard::Info(t) => {
            match t.get_interval() {
                (0, 1) => guard!(comp_sig["go"]),
                _ => unreachable!("This function is implemented for 1 cycle static components, only %0 can exist as timing guard"),

            }
        }
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
        fsm: &mut StaticFSM,
        builder: &mut ir::Builder,
        comp_sig: RRC<ir::Cell>,
    ) -> Vec<ir::Assignment<ir::Nothing>> {
        let first_state_guard = *fsm.query_between(builder, (0, 1));
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

    // Compiles `sgroup` according to the static component interface.
    // The assignments are removed from `sgroup` and placed into
    // `builder.component`'s continuous assignments.
    fn compile_static_interface(
        &self,
        sgroup: ir::RRC<ir::StaticGroup>,
        builder: &mut ir::Builder,
    ) {
        if sgroup.borrow().get_latency() > 1 {
            // Build a StaticSchedule object, realize it and add assignments
            // as continuous assignments.
            let mut sch = StaticSchedule::from(vec![Rc::clone(&sgroup)]);
            let (mut assigns, mut fsm) = sch.realize_schedule(builder, true);
            builder
                .component
                .continuous_assignments
                .extend(assigns.pop_front().unwrap());
            let comp_sig = Rc::clone(&builder.component.signature);
            if builder.component.attributes.has(ir::BoolAttr::Promoted) {
                // If necessary, add the logic to produce a done signal.
                let done_assigns =
                    Self::make_done_signal_for_promoted_component(
                        &mut fsm, builder, comp_sig,
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
            let assignments =
                std::mem::take(&mut sgroup.borrow_mut().assignments);
            for assign in assignments {
                let comp_sig = Rc::clone(&builder.component.signature);
                builder.component.continuous_assignments.push(
                    Self::make_assign_dyn_one_cycle_static_comp(
                        assign, comp_sig,
                    ),
                );
            }
            if builder.component.attributes.has(ir::BoolAttr::Promoted) {
                let comp_sig = Rc::clone(&builder.component.signature);
                let done_assigns =
                    Self::make_done_signal_for_promoted_component_one_cycle(
                        builder, comp_sig,
                    );
                builder
                    .component
                    .continuous_assignments
                    .extend(done_assigns);
            }
        }
    }
}

impl Visitor for CompileStatic {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Static components have a different interface than static groups.
        // If we have a static component, we have to compile the top-level
        // island (this island should be a group by now and corresponds
        // to the the entire control of the component) differently.
        // This island might invoke other static groups-- these static groups
        // should still follow the group interface.
        let top_level_sgroup = if comp.is_static() {
            let comp_control = comp.control.borrow();
            match &*comp_control {
                ir::Control::Static(ir::StaticControl::Enable(sen)) => {
                    Some(sen.group.borrow().name())
                }
                _ => return Err(Error::malformed_control(format!("Non-Enable Static Control should have been compiled away. Run {} to do this", crate::passes::StaticInliner::name()))),
            }
        } else {
            None
        };

        // Drain static groups of component
        let sgroups: Vec<ir::RRC<ir::StaticGroup>> =
            comp.get_static_groups_mut().drain().collect();

        // The first thing is to assign FSMs -> static islands.
        // We sometimes assign the same FSM to different static islands
        // to reduce register usage. We do this by getting greedy coloring.
        let coloring = Self::get_coloring(&sgroups, &comp.control.borrow());

        let mut builder = ir::Builder::new(comp, sigs);
        // Build one StaticSchedule object per color
        let mut schedule_objects =
            Self::build_schedule_objects(coloring, sgroups, &mut builder);

        // Map so we can rewrite `static_group[go]` to `early_reset_group[go]`
        let mut group_rewrites = ir::rewriter::PortRewriteMap::default();

        // Realize an fsm for each StaticSchedule object.
        for sch in &mut schedule_objects {
            // Check whether we are compiling the top level static island.
            let static_component_interface = match top_level_sgroup {
                None => false,
                // For the top level group, sch.static_groups should really only
                // have group--the top level group.
                Some(top_level_group) => sch
                    .static_groups
                    .iter()
                    .any(|g| g.borrow().name() == top_level_group),
            };
            // Static component/groups have different interfaces
            if static_component_interface {
                // Compile top level static group differently.
                // We know that the top level static island has its own
                // unique FSM so we can do `.pop().unwrap()`
                self.compile_static_interface(
                    sch.static_groups.pop().unwrap(),
                    &mut builder,
                )
            } else {
                let (mut static_group_assigns, fsm) = sch
                    .realize_schedule(&mut builder, static_component_interface);
                let fsm_ref = ir::rrc(fsm);
                for static_group in sch.static_groups.iter() {
                    // Create the dynamic "early reset group" that will replace the static group.
                    let static_group_name = static_group.borrow().name();
                    let mut early_reset_name = static_group_name.to_string();
                    early_reset_name.insert_str(0, "early_reset_");
                    let early_reset_group = builder.add_group(early_reset_name);
                    let mut assigns = static_group_assigns.pop_front().unwrap();

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
                    self.reset_early_map.insert(
                        static_group_name,
                        early_reset_group.borrow().name(),
                    );
                    // self.group_rewrite_map helps write static_group[go] to early_reset_group[go]
                    // Technically we could do this w/ early_reset_map but is easier w/
                    // group_rewrite, which is explicitly of type `PortRewriterMap`
                    group_rewrites.insert(
                        ir::Canonical::new(
                            static_group_name,
                            ir::Id::from("go"),
                        ),
                        early_reset_group.borrow().find("go").unwrap_or_else(
                            || {
                                unreachable!(
                                    "group {} has no go port",
                                    early_reset_group.borrow().name()
                                )
                            },
                        ),
                    );

                    self.fsm_info_map.insert(
                        early_reset_group.borrow().name(),
                        Rc::clone(&fsm_ref),
                    );
                }
            }
        }

        // Rewrite static_group[go] to early_reset_group[go]
        // don't have to worry about writing static_group[done] b/c static
        // groups don't have done holes.
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                group_rewrites
                    .get(&port.borrow().canonical())
                    .map(Rc::clone)
            })
        });

        // Add the static groups back to the component.
        for schedule in schedule_objects {
            comp.get_static_groups_mut()
                .append(schedule.static_groups.into_iter());
        }

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
        // No need to build wrapper for static component interface
        if comp.is_static() {
            return Ok(Action::Continue);
        }
        // Assume that there are only static enables left.
        // If there are any other type of static control, then error out.
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
                let fsm_object = self.fsm_info_map.get(early_reset_name).unwrap_or_else(|| unreachable!("group {} has no correspondoing fsm in self.fsm_map", early_reset_name));
                // If we've already made a wrapper for a group that uses the same
                // FSM, we can reuse the signal_reg. Otherwise, we must
                // instantiate a new signal_reg.
                let fsm_name = fsm_object.borrow().get_unique_id();
                let wrapper = match self.signal_reg_map.get(&fsm_name) {
                    None => {
                        // Need to build the signal_reg and the continuous
                        // assignment that resets the signal_reg
                        structure!( builder;
                            let signal_reg = prim std_reg(1);
                        );
                        self.signal_reg_map
                            .insert(fsm_name, signal_reg.borrow().name());
                        Self::build_wrapper_group(
                            Rc::clone(fsm_object),
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
                            Rc::clone(fsm_object),
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
                let fsm_object = self.fsm_info_map.get(reset_group_name).unwrap_or_else(|| unreachable!("group {} has no correspondoing fsm in self.fsm_map", reset_group_name));
                let wrapper_group = self.build_wrapper_group_while(
                    Rc::clone(fsm_object),
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
                unreachable!("Should have converted all static groups to dynamic. {} still has assignments in it. It's possible that you may need to run {} to remove dead groups and get rid of this error.", g.borrow().name(), crate::passes::DeadGroupRemoval::name());
            }
        }
        // remove all static groups
        comp.get_static_groups_mut().retain(|_| false);

        // Remove control if static component
        if comp.is_static() {
            comp.control = ir::rrc(ir::Control::empty())
        }

        Ok(Action::Continue)
    }
}
