use crate::analysis::{IncompleteTransition, StaticSchedule};
use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir::{self as ir, GetAttributes};
use calyx_utils::CalyxResult;
use core::ops::Not;
use itertools::Itertools;
const ACYCLIC: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::ACYCLIC);
const UNROLL: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::UNROLL);
const OFFLOAD: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::OFFLOAD);
const INLINE: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::INLINE);
const NUM_STATES: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NUM_STATES);

pub struct FSMBuilder {}

impl Named for FSMBuilder {
    fn name() -> &'static str {
        "fsm-builder"
    }
    fn description() -> &'static str {
        "translate control into structure using medium-sized explicit FSMs"
    }
}

impl ConstructVisitor for FSMBuilder {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(FSMBuilder {})
    }
    fn clear_data(&mut self) {}
}

pub struct Component {
    non_promoted_static_component: Option<bool>,
    static_control_component: bool,
    // In the future we'll want to incorporate dynamic components.
}

// Helper functions to get attributes for each part of the control.
/// Gets the `@ACYCLIC` attribute
fn is_acyclic<T: GetAttributes>(control: &T) -> bool {
    matches!(control.get_attributes().get(ACYCLIC), Some(1))
}
/// Gets the `@UNROLL` attribute
fn is_unroll<T: GetAttributes>(control: &T) -> bool {
    matches!(control.get_attributes().get(UNROLL), Some(1))
}
/// Gets the `@OFFLOAD` attribute
fn is_offload<T: GetAttributes>(control: &T) -> bool {
    matches!(control.get_attributes().get(OFFLOAD), Some(1))
}
/// Gets the `@INLINE` attribute
fn is_inline<T: GetAttributes>(control: &T) -> bool {
    matches!(control.get_attributes().get(INLINE), Some(1))
}

fn get_num_states(control: &ir::StaticControl) -> u64 {
    control.get_attribute(NUM_STATES).unwrap()
}

// A `StaticSchedule` is an abstract representation of fsms and maps out transitions, states, and assignments.
// This implmentation includes functions to build up static schedules and transform them to `ir::RRC::FSM`s.
impl StaticSchedule<'_, '_> {
    /// Provided a static control node, calling the `build_abstract` method on an empty `StaticSchedule`
    /// `sch` will build out the `latency` and `state2assigns` fields of `sch`, in
    /// preparation to replace the `StaticControl` node with an instance of `ir::FSM`.
    /// Every static assignment collected into `state2assigns` will have its existing guard
    /// "anded" with `guard`. The `looped_once_guard` is used to encode the "doneness" of a FSM.
    fn build_abstract(
        &mut self,
        scon: &ir::StaticControl,
        guard: ir::Guard<ir::Nothing>,
        mut transitions_to_curr: Vec<IncompleteTransition>,
        looped_once_guard: Option<ir::Guard<ir::Nothing>>,
    ) -> (Vec<IncompleteTransition>, Option<ir::Guard<ir::Nothing>>) {
        match scon {
            ir::StaticControl::Empty(_) => (transitions_to_curr, None),
            ir::StaticControl::Enable(sen) => {
                if is_acyclic(sen) && is_inline(sen) {
                    // @NUM_STATES(n) @ACYCLIC @INLINE
                    // The `@ACYCLIC` attribute requires that one state is allocated per cycle in a static enable.
                    // The `@INLINE` attribute requires that this node must allocate states for this enable.
                    // For all parts of the FSM that want to transition to this enable,
                    // register their transitions in self.state2trans.
                    self.register_transitions(
                        self.state,
                        &mut transitions_to_curr,
                        guard.clone(),
                    );

                    sen.group.borrow().assignments.iter().for_each(|sassign| {
                        sassign
                            .guard
                            .compute_live_states(sen.group.borrow().latency)
                            .into_iter()
                            .for_each(|offset| {
                                // convert the static assignment to a normal one
                                let mut assign: ir::Assignment<ir::Nothing> =
                                    ir::Assignment::from(sassign.clone());
                                // "and" the assignment's guard with argument guard
                                assign.and_guard(guard.clone());
                                // add this assignment to the list of assignments
                                // that are supposed to be valid at this state
                                self.state2assigns
                                    .entry(self.state + offset)
                                    .and_modify(|other_assigns| {
                                        other_assigns.push(assign.clone())
                                    })
                                    .or_insert(vec![assign]);
                            })
                    });
                    // On an acyclic annotated node, we allocate N states to make N cycles elapse.
                    self.state += sen.group.borrow().latency;
                    // Don't know where to transition next; let the parent that called
                    // `build_abstract` deal with registering the transition
                    // from the state(s) we just built.
                    (
                        vec![IncompleteTransition::new(
                            self.state - 1,
                            ir::Guard::True,
                        )],
                        looped_once_guard,
                    )
                } else if is_inline(sen) {
                    // @NUM_STATES(n) @INLINE
                    // In the absence of `@ACYCLIC`, the node must contain cycles,
                    // or have children that contain cycles
                    // We should create `n` states.
                    // We'll run this placeholder code that creates one state for now.
                    self.register_transitions(
                        self.state,
                        &mut transitions_to_curr,
                        guard.clone(),
                    );

                    let final_state_guard =
                        self.leave_one_state_condition(guard, sen);

                    self.state += 1;
                    (
                        vec![IncompleteTransition::new(
                            self.state - 1,
                            final_state_guard,
                        )],
                        None,
                    )
                } else {
                    unreachable!(
                        "`build_abstract` encountered a node without any annotations."
                    )
                }
            }
            ir::StaticControl::Seq(sseq) => {
                if is_acyclic(sseq) && is_inline(sseq) {
                    // @NUM_STATES(n) @ACYCLIC @INLINE
                    (
                        sseq.stmts.iter().enumerate().fold(
                            transitions_to_curr,
                            |transitions_to_this_stmt, (_, stmt)| {
                                self.build_abstract(
                                    stmt,
                                    guard.clone(),
                                    transitions_to_this_stmt,
                                    looped_once_guard.clone(),
                                )
                                .0
                            },
                        ),
                        None,
                    )
                } else if is_inline(sseq) {
                    // @NUM_STATES(n) @INLINE
                    // may be incorrect for now, must think about what back edges might be possible for a static seq
                    (
                        sseq.stmts.iter().fold(
                            transitions_to_curr,
                            |transitions_to_this_stmt, stmt| {
                                self.build_abstract(
                                    stmt,
                                    guard.clone(),
                                    transitions_to_this_stmt,
                                    looped_once_guard.clone(),
                                )
                                .0
                            },
                        ),
                        None,
                    )
                } else if is_offload(sseq) {
                    // @NUM_STATES(1) @OFFLOAD
                    unreachable!(
                        "`build_abstract` encountered an impossible offload of Static Seq node."
                    )
                } else {
                    // cyclic static seqs are not possible
                    // we must have at least one `attr` annotation
                    unreachable!(
                        "`build_abstract` encountered a node without any annotations."
                    )
                }
            }
            ir::StaticControl::Repeat(srep) => {
                if is_acyclic(srep) && is_unroll(srep) {
                    // @ACYCLIC @UNROLL
                    // In the encounter of a `@UNROLL` attribute, we'll want to create a state for each child.
                    (
                        (0..srep.num_repeats).fold(
                            transitions_to_curr,
                            |transitions_to_this_body, _| {
                                self.build_abstract(
                                    &srep.body,
                                    guard.clone(),
                                    transitions_to_this_body,
                                    looped_once_guard.clone(),
                                )
                                .0
                            },
                        ),
                        None,
                    )
                } else if is_offload(srep) {
                    // @NUM_STATES(1) @OFFLOAD
                    // In the case of offload, we'll want to create a state with a register to count the number of
                    // times to loop in place
                    unreachable!(
                        "`build_abstract` offload of `static_repeat` nodes should have been transformed away."
                    )
                } else if is_inline(srep) {
                    // @NUM_STATES(n) @INLINE
                    // Create a loop: the body has n states (from annotations)
                    // We build those states once, add a counter, and create a back edge

                    // Register incoming transitions to start of repeat
                    self.register_transitions(
                        self.state,
                        &mut transitions_to_curr,
                        guard.clone(),
                    );

                    let loop_start_state = self.state;

                    // Get the number of states the body needs from its annotation
                    let body_num_states = get_num_states(&srep.body);

                    // Build the body ONCE to populate the state->assignments mapping
                    let (_body_exits, _) = self.build_abstract(
                        &srep.body,
                        guard.clone(),
                        vec![],
                        looped_once_guard.clone(),
                    );

                    // After building the body, self.state has advanced by body_num_states
                    // So the last state of the loop body is self.state - 1
                    let loop_end_state = loop_start_state + body_num_states - 1;

                    // Create a counter to track iterations
                    let counter_width =
                        calyx_utils::math::bits_needed_for(srep.num_repeats);
                    let counter = self.builder.add_primitive(
                        format!("repeat_counter_{loop_start_state}"),
                        "std_reg",
                        &[counter_width],
                    );
                    counter
                        .borrow_mut()
                        .add_attribute(ir::BoolAttr::FSMControl, 1);

                    let signal_on = self.builder.add_constant(1, 1);
                    let counter_max = self
                        .builder
                        .add_constant(srep.num_repeats - 1, counter_width);

                    // Increment counter on the last state of the loop body
                    let incr = self.builder.add_primitive(
                        format!("repeat_incr_{loop_start_state}"),
                        "std_add",
                        &[counter_width],
                    );
                    incr.borrow_mut()
                        .add_attribute(ir::BoolAttr::FSMControl, 1);

                    let one = self.builder.add_constant(1, counter_width);

                    // Assignments to increment the counter
                    let incr_assigns = vec![
                        self.builder.build_assignment(
                            incr.borrow().get("left"),
                            counter.borrow().get("out"),
                            ir::Guard::True,
                        ),
                        self.builder.build_assignment(
                            incr.borrow().get("right"),
                            one.borrow().get("out"),
                            ir::Guard::True,
                        ),
                        self.builder.build_assignment(
                            counter.borrow().get("in"),
                            incr.borrow().get("out"),
                            ir::Guard::True,
                        ),
                        self.builder.build_assignment(
                            counter.borrow().get("write_en"),
                            signal_on.borrow().get("out"),
                            ir::Guard::True,
                        ),
                    ];

                    // Add increment assignments to the last state of the body
                    self.state2assigns
                        .entry(loop_end_state)
                        .and_modify(|assigns| {
                            assigns.extend(incr_assigns.clone())
                        })
                        .or_insert(incr_assigns);

                    // Create guard: counter < num_repeats - 1 (loop condition)
                    let lt = self.builder.add_primitive(
                        format!("repeat_lt_{loop_start_state}"),
                        "std_lt",
                        &[counter_width],
                    );
                    lt.borrow_mut().add_attribute(ir::BoolAttr::FSMControl, 1);

                    let loop_cond_assigns = vec![
                        self.builder.build_assignment(
                            lt.borrow().get("left"),
                            counter.borrow().get("out"),
                            ir::Guard::True,
                        ),
                        self.builder.build_assignment(
                            lt.borrow().get("right"),
                            counter_max.borrow().get("out"),
                            ir::Guard::True,
                        ),
                    ];

                    // These assignments should be continuous
                    self.builder.add_continuous_assignments(loop_cond_assigns);

                    // Create the loop-back transition: if counter < max, go back to loop start
                    let loop_back_guard =
                        ir::Guard::port(lt.borrow().get("out"));
                    let loop_back_transition = IncompleteTransition::new(
                        loop_end_state,
                        loop_back_guard.clone(),
                    );

                    // Register the back edge
                    self.register_transitions(
                        loop_start_state,
                        &mut vec![loop_back_transition],
                        guard.clone(),
                    );

                    // Exit condition: counter >= max, exit the loop
                    let exit_guard =
                        ir::Guard::Not(Box::new(loop_back_guard.clone()));

                    // Return transition from the final state when loop is done
                    (
                        vec![IncompleteTransition::new(
                            loop_end_state,
                            exit_guard,
                        )],
                        None,
                    )
                } else {
                    // we must have at least one `attr` annotation
                    unreachable!(
                        "`build_abstract` encountered a node without any annotations."
                    )
                }
            }
            ir::StaticControl::If(sif) => {
                if is_acyclic(sif) && is_inline(sif) {
                    // @NUM_STATES(n) @ACYCLIC @INLINE
                    // Both branches execute in parallel with conditional guards
                    let true_guard =
                        guard.clone().and(ir::Guard::port(sif.port.clone()));
                    let false_guard = guard
                        .clone()
                        .and(ir::Guard::not(ir::Guard::port(sif.port.clone())));

                    // Build both branches with their respective guards
                    (
                        self.build_abstract(
                            &sif.tbranch,
                            true_guard,
                            transitions_to_curr.clone(),
                            looped_once_guard.clone(),
                        )
                        .0
                        .into_iter()
                        .chain(
                            self.build_abstract(
                                &sif.fbranch,
                                false_guard,
                                transitions_to_curr,
                                looped_once_guard,
                            )
                            .0,
                        )
                        .collect(),
                        None,
                    )
                } else if is_offload(sif) {
                    // @NUM_STATES(1) @OFFLOAD
                    unreachable!(
                        "`build_abstract` encountered an impossible offload of Static Seq node."
                    )
                } else {
                    // we must have at least one `attr` annotation
                    unreachable!(
                        "`build_abstract` encountered a node without any annotations."
                    )
                }
            }
            ir::StaticControl::Par(spar) => {
                if is_acyclic(spar) && is_inline(spar) {
                    // @NUM_STATES(n) @ACYCLIC @INLINE
                    // schedule children in lock-step.
                    // we register incoming transitions to the start state of the par region
                    // for each child, build its schedule into a temporary StaticSchedule
                    // (so the child's states start at 0 in the tmp schedule).
                    // then we collect each child's per-state assignments and merge them 
                    // into `self.state2assigns` aligned at `self.state`.
                    // and finally reserve `L = max_i latency(i)` states in `self.state` 
                    // and return a single exit transition from `self.state + L - 1`.

                    // Register incoming transitions to the start of the par region
                    self.register_transitions(
                        self.state,
                        &mut transitions_to_curr,
                        guard.clone(),
                    );

                    let par_start = self.state;

                    // Build temporary schedules and collect per-thread maps
                    let mut max_len: u64 = 0;
                    let mut per_thread_maps: Vec<
                        std::collections::HashMap<
                            u64,
                            Vec<ir::Assignment<ir::Nothing>>,
                        >,
                    > = Vec::new();

                    for thread in spar.stmts.iter() {
                        // Build the thread in a temporary schedule so we can observe
                        // its per-state assignments without advancing `self.state`.
                        let mut tmp = StaticSchedule::from(&mut *self.builder);
                        let (_exits, _g) = tmp.build_abstract(
                            thread,
                            guard.clone(),
                            vec![],
                            looped_once_guard.clone(),
                        );

                        let thread_len = get_num_states(thread);
                        max_len = max_len.max(thread_len);

                        // Drain the tmp schedule's state->assigns map into a HashMap
                        let map: std::collections::HashMap<
                            u64,
                            Vec<ir::Assignment<ir::Nothing>>,
                        > = tmp.state2assigns.drain().collect();
                        per_thread_maps.push(map);
                    }

                    // Merge per-thread maps into the current schedule, aligned at par_start
                    for map in per_thread_maps.into_iter() {
                        for (s, assigns) in map.into_iter() {
                            let target = par_start + s;
                            self.state2assigns
                                .entry(target)
                                .and_modify(|other| {
                                    other.extend(assigns.clone())
                                })
                                .or_insert(assigns.clone());
                        }
                    }

                    // Reserve max_len states and return exit transition from final state
                    if max_len == 0 {
                        // empty par -> immediate exit
                        (
                            vec![IncompleteTransition::new(
                                par_start,
                                ir::Guard::True,
                            )],
                            None,
                        )
                    } else {
                        self.state += max_len;
                        (
                            vec![IncompleteTransition::new(
                                self.state - 1,
                                ir::Guard::True,
                            )],
                            None,
                        )
                    }
                } else if is_offload(spar) {
                    // @NUM_STATES(1) @OFFLOAD
                    unreachable!(
                        "`build_abstract` encountered an impossible offload of Static Seq node."
                    )
                } else {
                    // we must have at least one `attr` annotation
                    unreachable!(
                        "`build_abstract` encountered a node without any annotations."
                    )
                }
            }
            ir::StaticControl::Invoke(_) => {
                unreachable!(
                    "`build_abstract` encountered a `static_invoke` node. \
              Should have been compiled away."
                )
            }
        }
    }
    /// Returns the FSM implementing the given control node, as well as the builder
    /// object from which it was built.
    fn fsm_build(
        &mut self,
        control: &ir::StaticControl,
        build_component_type: Component, // need to get better type name. Some(True) means non-promoted-static-component. False means promoted/static island. Otherwise it's dynamic
    ) -> ir::RRC<ir::FSM> {
        let signal_on = self.builder.add_constant(1, 1);

        // Declare the FSM
        let fsm = self.builder.add_fsm("fsm");

        let (mut remaining_assignments, additional_looped_once_guard) =
            self.build_abstract(control, ir::Guard::True, vec![], None);

        // add loopback transitions to first state
        self.register_transitions(
            0,
            &mut remaining_assignments,
            ir::Guard::True,
        );

        let (mut assignments, transitions, state2wires) =
            self.build_fsm_pieces(ir::RRC::clone(&fsm));

        // We'll build the fsm different based off of what kind of component this node is.
        match build_component_type {
            Component {
                non_promoted_static_component: Some(true),
                static_control_component: true,
            } => {
                // If the component is static by design, there will be exactly one
                // FSM allocated to it. We will get rid of the FSMEnable node from the
                // control in this case, so we need to manually add fsm[start] = comp[go]
                // because wire-inliner will not get to it.

                // (We get rid of the FSMEnable node because the FSM will not have a
                // DONE state, and hence no way to terminate the control. )
                let assign_fsm_start = self.builder.build_assignment(
                    fsm.borrow().get("start"),
                    self.builder
                        .component
                        .signature
                        .borrow()
                        .find_unique_with_attr(ir::NumAttr::Go)
                        .unwrap()
                        .unwrap(),
                    ir::Guard::True,
                );
                self.builder
                    .add_continuous_assignments(vec![assign_fsm_start]);
            }
            Component {
                non_promoted_static_component: Some(false),
                static_control_component: true,
            } => {
                // In this case, the component is either a promoted static component
                // or the control is a static island that needs to handshake with its
                // surrounding dynamic context. In either event, we want to assign
                // fsm[done] to maintain the dynamic interface. We'll do this in state 0:

                // register to store whether the FSM has been run exactly one time when
                // we return to state 0
                let looped_once: ir::RRC<ir::Cell> =
                    self.builder.add_primitive("looped_once", "std_reg", &[1]);

                looped_once
                    .borrow_mut()
                    .add_attribute(ir::BoolAttr::FSMControl, 1);

                let (assign_looped_once, assign_looped_once_we, fsm_done) = (
                    self.builder.build_assignment(
                        looped_once.borrow().get("in"),
                        signal_on.borrow().get("out"),
                        match additional_looped_once_guard {
                            None => ir::guard!(fsm["start"]),
                            Some(g) => ir::guard!(fsm["start"]).and(g),
                        },
                    ),
                    self.builder.build_assignment(
                        looped_once.borrow().get("write_en"),
                        signal_on.borrow().get("out"),
                        ir::Guard::True,
                    ),
                    self.builder.build_assignment(
                        fsm.borrow().get("done"),
                        looped_once.borrow().get("out"),
                        ir::Guard::True,
                    ),
                );

                assignments.first_mut().unwrap().extend(vec![
                    assign_looped_once,
                    assign_looped_once_we,
                    fsm_done,
                ]);
            }
            Component {
                non_promoted_static_component: None,
                static_control_component: true,
            } => {
                // Do nothing because we want to build a subset of static control component.
                // Think ifs, repeats, pars, which don't rely on doneness.
            }
            Component {
                non_promoted_static_component: _, // This branch doesn't matter in a dynamic component.
                static_control_component: false,
            } => {
                todo!("Dynamic component!")
            }
        }

        // Build up the fsm here and return.

        // For test cases, we want to maintain a reliable order!
        let mut state_assigns: Vec<_> = self.state2assigns.drain().collect();
        state_assigns.sort_by_key(|(state, _)| *state);

        // Build up the fsm here and return.
        self.builder.add_continuous_assignments(
            state_assigns
                .into_iter()
                .flat_map(|(state, mut assigns)| {
                    assigns.iter_mut().for_each(|assign| {
                        assign.and_guard(ir::Guard::port(
                            state2wires
                                .get(state as usize)
                                .unwrap()
                                .borrow()
                                .get("out"),
                        ));
                    });
                    assigns
                })
                .collect(),
        );

        // Instantiate the FSM with the assignments and transitions we built.
        fsm.borrow_mut().extend_fsm(assignments, transitions);
        fsm
    }
}

impl Visitor for FSMBuilder {
    fn finish_static_repeat(
        &mut self,
        s: &mut calyx_ir::StaticRepeat,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        if is_offload(s) {
            let non_promoted_static_component = comp.is_static()
                && !(comp
                    .attributes
                    .has(ir::Attribute::Bool(ir::BoolAttr::Promoted)));

            let mut builder = ir::Builder::new(comp, sigs);
            let signal_on = builder.add_constant(1, 1);
            let repeat_group = builder.add_static_group("repeat", s.latency);
            let mut sch_generator = StaticSchedule::from(&mut builder);

            let trigger_fsm = {
                // This FSM implements the schedule for the body of the repeat
                let fsm = sch_generator.fsm_build(
                    &s.body,
                    Component {
                        non_promoted_static_component: Some(
                            non_promoted_static_component,
                        ),
                        static_control_component: true,
                    },
                );

                let mut trigger_thread = builder.build_assignment(
                    fsm.borrow().get("start"),
                    signal_on.borrow().get("out"),
                    ir::Guard::True,
                );
                // Make fsm[start] active for the entire execution of the repeat,
                // not just the first cycle. This way, we can repeat the body the desired
                // number of times.
                trigger_thread
                    .guard
                    .add_interval(ir::StaticTiming::new((0, s.latency)));
                trigger_thread
            };

            repeat_group.borrow_mut().assignments.push(trigger_fsm);
            let mut enable = ir::StaticControl::Enable(ir::StaticEnable {
                group: repeat_group,
                attributes: ir::Attributes::default(),
            });
            enable.get_mut_attributes().insert(INLINE, 1);
            Ok(Action::static_change(enable))
        } else {
            Ok(Action::Continue)
        }
    }

    fn finish_static_if(
        &mut self,
        s: &mut calyx_ir::StaticIf,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        if is_offload(s) {
            let non_promoted_static_component = comp.is_static()
                && !(comp
                    .attributes
                    .has(ir::Attribute::Bool(ir::BoolAttr::Promoted)));

            let mut builder = ir::Builder::new(comp, sigs);
            let signal_on = builder.add_constant(1, 1);

            // generate FSM for true branch
            let mut sch_constructor_true = StaticSchedule::from(&mut builder);
            let true_branch_fsm = sch_constructor_true.fsm_build(
                &s.tbranch,
                Component {
                    non_promoted_static_component: Some(
                        non_promoted_static_component,
                    ),
                    static_control_component: true,
                },
            );

            // group to active each FSM conditionally
            let if_group = builder.add_static_group("if", s.latency);
            let true_guard: ir::Guard<ir::StaticTiming> =
                ir::Guard::port(ir::RRC::clone(&s.port));
            let false_guard = ir::Guard::not(true_guard.clone());

            // assignments to active each FSM
            let mut trigger_fsms_with_branch_latency = vec![(
                builder.build_assignment(
                    true_branch_fsm.borrow().get("start"),
                    signal_on.borrow().get("out"),
                    true_guard,
                ),
                s.tbranch.get_latency(),
            )];

            // generate FSM and start condition for false branch if branch not empty
            if !(matches!(&*s.fbranch, ir::StaticControl::Empty(_))) {
                let mut sch_constructor_false =
                    StaticSchedule::from(&mut builder);
                let false_branch_fsm = sch_constructor_false.fsm_build(
                    &s.fbranch,
                    Component {
                        non_promoted_static_component: Some(
                            non_promoted_static_component,
                        ),
                        static_control_component: true,
                    },
                );
                trigger_fsms_with_branch_latency.push((
                    builder.build_assignment(
                        false_branch_fsm.borrow().get("start"),
                        signal_on.borrow().get("out"),
                        false_guard,
                    ),
                    s.fbranch.get_latency(),
                ));
            }

            // make sure [start] for each FSM is pulsed at most once, at the first
            // cycle

            let trigger_fsms = trigger_fsms_with_branch_latency
                .into_iter()
                .map(|(mut assign, latency)| {
                    assign
                        .guard
                        .add_interval(ir::StaticTiming::new((0, latency)));
                    assign
                })
                .collect_vec();

            if_group.borrow_mut().assignments.extend(trigger_fsms);

            // ensure this group only gets one state in the parent FSM, and only
            // transitions out when the latency counter has completed
            let mut enable = ir::StaticControl::Enable(ir::StaticEnable {
                group: if_group,
                attributes: ir::Attributes::default(),
            });
            // enable
            //     .get_mut_attributes()
            //     .insert(ir::BoolAttr::OneState, 1);

            enable.get_mut_attributes().insert(INLINE, 1);
            Ok(Action::static_change(enable))
        } else {
            Ok(Action::Continue)
        }
    }

    fn finish_static_par(
        &mut self,
        spar: &mut calyx_ir::StaticPar,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        let non_promoted_static_component = comp.is_static()
            && !(comp
                .attributes
                .has(ir::Attribute::Bool(ir::BoolAttr::Promoted)));
        if is_offload(spar) {
            let mut builder = ir::Builder::new(comp, sigs);
            let signal_on = builder.add_constant(1, 1);
            let par_group = builder.add_static_group("par", spar.latency);
            par_group
                .borrow_mut()
                .assignments
                .extend(spar.stmts.iter().map(|thread: &ir::StaticControl| {
                    let mut sch_generator = StaticSchedule::from(&mut builder);
                    let thread_latency = thread.get_latency();
                    let thread_fsm = sch_generator.fsm_build(
                        thread,
                        Component {
                            non_promoted_static_component: Some(
                                non_promoted_static_component,
                            ),
                            static_control_component: true,
                        },
                    );
                    let mut trigger_thread = builder.build_assignment(
                        thread_fsm.borrow().get("start"),
                        signal_on.borrow().get("out"),
                        ir::Guard::True,
                    );
                    trigger_thread.guard.add_interval(ir::StaticTiming::new((
                        0,
                        thread_latency,
                    )));
                    trigger_thread
                }));

            let mut enable = ir::StaticControl::Enable(ir::StaticEnable {
                group: par_group,
                attributes: ir::Attributes::default(),
            });
            // enable
            //     .get_mut_attributes()
            //     .insert(ir::BoolAttr::OneState, 1);
            enable.get_mut_attributes().insert(INLINE, 1);

            Ok(Action::static_change(enable))
        } else {
            Ok(Action::Continue)
        }
    }
    /// `finish_static_control` is called once, at the very end of traversing the control tree,
    /// when all child nodes have been traversed. We traverse the static control node from parent to
    /// child, and recurse inward to inline children.
    fn finish_static_control(
        &mut self,
        scon: &mut calyx_ir::StaticControl,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        let non_promoted_static_component = comp.is_static()
            && !(comp
                .attributes
                .has(ir::Attribute::Bool(ir::BoolAttr::Promoted)));

        // Implementation for single static enable components and static seqs for now.
        let mut builder = ir::Builder::new(comp, sigs);

        let mut ssch = StaticSchedule::from(&mut builder);

        Ok(Action::change(ir::Control::fsm_enable(ssch.fsm_build(
            scon,
            Component {
                non_promoted_static_component: Some(
                    non_promoted_static_component,
                ),
                static_control_component: true,
            },
        ))))
    }
}
