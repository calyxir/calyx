use super::math_utilities::get_bit_width_from;
use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir::{self as ir, build_assignments, guard};
use calyx_utils::CalyxResult;
use core::ops::Not;
use itertools::Itertools;
use std::collections::HashMap;

pub struct StaticFSMAllocation {
    non_promoted_static_component: bool,
}

impl Named for StaticFSMAllocation {
    fn name() -> &'static str {
        "static-fsm-alloc"
    }
    fn description() -> &'static str {
        "compiles a static schedule into an FSM construct"
    }
}

impl ConstructVisitor for StaticFSMAllocation {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(StaticFSMAllocation {
            non_promoted_static_component: false,
        })
    }
    fn clear_data(&mut self) {
        self.non_promoted_static_component = false
    }
}

/// Represents an FSM transition that doesn't yet have a destination state.
#[derive(Clone)]
struct IncompleteTransition {
    source: u64,
    guard: ir::Guard<ir::Nothing>,
}

impl IncompleteTransition {
    fn new(source: u64, guard: ir::Guard<ir::Nothing>) -> Self {
        Self { source, guard }
    }
}

/// An instance of `StaticSchedule` is constrainted to live at least as long as
/// the component in which the static island that it represents lives.
struct StaticSchedule<'b, 'a: 'b> {
    /// Builder construct to add hardware to the component it's built from
    builder: &'b mut ir::Builder<'a>,
    /// Number of cycles to which the static schedule should count up
    state: u64,
    /// Maps every FSM state to assignments that should be active in that state
    state2assigns: HashMap<u64, Vec<ir::Assignment<ir::Nothing>>>,
    /// Parital map from FSM state to transitions out of that state.
    /// If a state has no mapping, assume it's an unconditional transition to
    /// state + 1.
    state2trans: HashMap<u64, ir::Transition>,
}

impl<'b, 'a> From<&'b mut ir::Builder<'a>> for StaticSchedule<'b, 'a> {
    fn from(builder: &'b mut ir::Builder<'a>) -> Self {
        StaticSchedule {
            builder: builder,
            state: 0,
            state2assigns: HashMap::new(),
            state2trans: HashMap::new(),
        }
    }
}

impl<'b, 'a> StaticSchedule<'b, 'a> {
    fn register_transitions(
        &mut self,
        curr_state: u64,
        transitions_to_curr: &mut Vec<IncompleteTransition>,
        and_guard: ir::Guard<ir::Nothing>,
    ) {
        transitions_to_curr.drain(..).for_each(
            |IncompleteTransition { source, guard }| {
                let complete_transition =
                    match (guard, &and_guard) {
                        (ir::Guard::True, ir::Guard::True) => {
                            ir::Transition::Unconditional(curr_state)
                        }
                        (ir::Guard::True, _) => ir::Transition::Conditional(
                            vec![(and_guard.clone(), curr_state)],
                        ),
                        (guard, ir::Guard::True) => {
                            ir::Transition::Conditional(vec![(
                                guard, curr_state,
                            )])
                        }
                        (guard, and_guard) => ir::Transition::Conditional(
                            vec![(guard.and(and_guard.clone()), curr_state)],
                        ),
                    };

                self.state2trans
                    .entry(source)
                    .and_modify(|existing_transition| {
                        match (existing_transition, complete_transition.clone())
                        {
                            (ir::Transition::Unconditional(_), _)
                            | (_, ir::Transition::Unconditional(_)) => (),
                            (
                                ir::Transition::Conditional(existing_conds),
                                ir::Transition::Conditional(new_conds),
                            ) => {
                                existing_conds.extend(new_conds);
                            }
                        };
                    })
                    .or_insert(complete_transition);
            },
        );
    }

    /// Provided a static control node, calling this method on an empty `StaticSchedule`
    /// `sch` will build out the `latency` and `state2assigns` fields of `sch`, in
    /// preparation to replace the `StaticControl` node with an instance of `ir::FSM`.
    /// Every static assignment collected into `state2assigns` will have its existing guard
    /// "anded" with `guard`.
    fn build_abstract_fsm(
        &mut self,
        scon: &ir::StaticControl,
        guard: ir::Guard<ir::Nothing>,
        mut transitions_to_curr: Vec<IncompleteTransition>,
        looped_once_guard: Option<ir::Guard<ir::Nothing>>,
    ) -> (Vec<IncompleteTransition>, Option<ir::Guard<ir::Nothing>>) {
        match scon {
            ir::StaticControl::Empty(_) => {
                (transitions_to_curr, looped_once_guard)
            }
            ir::StaticControl::Enable(sen) => {
                // for all parts of the FSM that want to transition to this enable,
                // register their transitions in self.state2trans
                self.register_transitions(
                    self.state,
                    &mut transitions_to_curr,
                    guard.clone(),
                );

                // allocate one state if requested, and have one state for every
                // cycle otherwise
                if sen.attributes.has(ir::BoolAttr::OneState) {
                    let signal_on = self.builder.add_constant(1, 1);
                    let group_latency = sen.group.borrow().get_latency();

                    // instantiate a local counter register
                    let width = get_bit_width_from(group_latency);
                    let counter = self.builder.add_primitive(
                        "group_counter",
                        "std_reg",
                        &[width],
                    );

                    // transform all assignments in the static group to read
                    // from the local counter
                    let mut assigns: Vec<ir::Assignment<ir::Nothing>> = sen
                        .group
                        .borrow_mut()
                        .assignments
                        .clone()
                        .drain(..)
                        .map(|mut sassign| {
                            sassign.guard.replace_static_timing(
                                self.builder,
                                &counter,
                                &width,
                            );
                            let mut assign = ir::Assignment::from(sassign);
                            assign.and_guard(guard.clone());
                            assign
                        })
                        .collect();

                    // guard reprsenting if counter is in final state
                    let final_state_const =
                        self.builder.add_constant(group_latency - 1, width);
                    let final_state_wire: ir::RRC<ir::Cell> =
                        self.builder.add_primitive(
                            format!("const{}_{}_", group_latency - 1, width),
                            "std_wire",
                            &[width],
                        );
                    let final_state_guard = ir::Guard::CompOp(
                        ir::PortComp::Eq,
                        counter.borrow().get("out"),
                        final_state_wire.borrow().get("out"),
                    );
                    let not_final_state_guard = final_state_guard.clone().not();

                    // build assignments to increment / reset the counter
                    let adder = self.builder.add_primitive(
                        "adder",
                        "std_add",
                        &[width],
                    );
                    let const_one = self.builder.add_constant(1, width);
                    let const_zero = self.builder.add_constant(0, width);
                    let incr_counter_assigns = build_assignments!(self.builder;
                        final_state_wire["in"] = ? final_state_const["out"];
                        adder["left"] = ? counter["out"];
                        adder["right"] = ? const_one["out"];
                        counter["write_en"] = ? signal_on["out"];
                        counter["in"] = final_state_guard ? const_zero["out"];
                        counter["in"] = not_final_state_guard ? adder["out"];
                    );

                    assigns.extend(incr_counter_assigns.to_vec());

                    // push these assignments into the one state allocated for this
                    // enable
                    self.state2assigns
                        .entry(self.state)
                        .and_modify(|other_assigns| {
                            other_assigns.extend(assigns.clone());
                        })
                        .or_insert(assigns);

                    let new_looped_once_guard = match self.state {
                        0 => Some(final_state_guard.clone()),
                        _ => looped_once_guard,
                    };
                    self.state += 1;
                    (
                        vec![IncompleteTransition::new(
                            self.state - 1,
                            final_state_guard,
                        )],
                        new_looped_once_guard,
                    )
                } else {
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
                    self.state += sen.group.borrow().latency;
                    // Don't know where to transition next; let the parent that called
                    // `build_abstract_fsm` deal with registering the transition from the state(s)
                    // we just built.
                    (
                        vec![IncompleteTransition::new(
                            self.state - 1,
                            ir::Guard::True,
                        )],
                        looped_once_guard,
                    )
                }
            }
            ir::StaticControl::Seq(sseq) => sseq.stmts.iter().fold(
                (transitions_to_curr, looped_once_guard),
                |(transitions_to_this_stmt, looped_once_guard_this_stmt),
                 stmt| {
                    self.build_abstract_fsm(
                        stmt,
                        guard.clone(),
                        transitions_to_this_stmt,
                        looped_once_guard_this_stmt,
                    )
                },
            ),

            ir::StaticControl::If(_) => {
                unreachable!(
                    "`construct_schedule` encountered a `static_if` node. \
              Should have been compiled into a static group."
                )
            }
            ir::StaticControl::Repeat(_) => {
                unreachable!(
                    "`construct_schedule` encountered a `static_repeat` node. \
              Should have been compiled into a static group."
                )
            }
            ir::StaticControl::Par(_) => {
                unreachable!(
                    "`construct_schedule` encountered a `static_par` node. \
              Should have been compiled into a static group."
                )
            }
            ir::StaticControl::Invoke(_) => {
                unreachable!(
                    "`construct_schedule` encountered a `static_invoke` node. \
              Should have been compiled away."
                )
            }
        }
    }

    /// Given a filled-out static schedule, construct an FSM based on the state mappings
    /// in `state2assigns`.
    fn realize_fsm(
        &mut self,
        control: &ir::StaticControl,
        non_promoted_static_component: bool,
    ) -> ir::RRC<ir::FSM> {
        let true_guard = ir::Guard::True;
        let signal_on = self.builder.add_constant(1, 1);

        // Declare the FSM
        let fsm = self.builder.add_fsm("fsm");

        let (mut remaining_assignments, additional_looped_once_guard) =
            self.build_abstract_fsm(control, ir::Guard::True, vec![], None);

        // add loopback transitions to first state
        self.register_transitions(
            0,
            &mut remaining_assignments,
            ir::Guard::True,
        );

        let (mut assignments, transitions, state2wires): (
            Vec<Vec<ir::Assignment<ir::Nothing>>>,
            Vec<ir::Transition>,
            Vec<ir::RRC<ir::Cell>>,
        ) = (0..self.state)
            .map(|state| {
                // construct a wire to represent this state
                let state_wire: ir::RRC<ir::Cell> = self.builder.add_primitive(
                    format!("{}_{state}", fsm.borrow().name().to_string()),
                    "std_wire",
                    &[1],
                );
                // build assignment to indicate that we're in this state
                let mut state_assign: ir::Assignment<ir::Nothing> =
                    self.builder.build_assignment(
                        state_wire.borrow().get("in"),
                        signal_on.borrow().get("out"),
                        ir::Guard::True,
                    );

                // merge `idle` and first `calc` state
                if state == 0 {
                    state_assign.and_guard(ir::guard!(fsm["start"]));
                }

                // let assigns_at_state = match self.state2assigns.remove(&state) {
                //     None => vec![],
                //     Some(mut assigns) => {
                //         // merge `idle` and first `calc` state
                //         if state == 0 {
                //             assigns.iter_mut().for_each(|assign| {
                //                 assign.and_guard(guard!(fsm["start"]));
                //             });
                //         }
                //         assigns
                //     }
                // };

                let transition_from_state = match self
                    .state2trans
                    .remove(&state)
                {
                    Some(mut transition) => {
                        // if in first state, transition conditioned on fsm[start]
                        let transition_mut_ref = &mut transition;
                        if state == 0 {
                            match transition_mut_ref {
                                ir::Transition::Unconditional(next_state) => {
                                    *transition_mut_ref =
                                        ir::Transition::Conditional(vec![
                                            (guard!(fsm["start"]), *next_state),
                                            (ir::Guard::True, 0),
                                        ]);
                                }
                                ir::Transition::Conditional(conditions) => {
                                    conditions.iter_mut().for_each(
                                        |(condition, _)| {
                                            condition.update(|g| {
                                                g.and(guard!(fsm["start"]))
                                            });
                                        },
                                    );
                                }
                            }
                        }

                        // add a default self-loop for every conditional transition
                        // if it doesn't already have it
                        if let ir::Transition::Conditional(trans) =
                            &mut transition
                        {
                            if !(trans.last_mut().unwrap().0.is_true()) {
                                trans.push((ir::Guard::True, state))
                            }
                        }
                        transition
                    }
                    None => {
                        if state == 0 {
                            // set transition out of first state, which is
                            // conditional on reading fsm[start]
                            ir::Transition::Conditional(vec![
                                (guard!(fsm["start"]), 1 % self.state),
                                (ir::Guard::True, 0),
                            ])
                        } else {
                            // loopback to start at final state, and increment
                            // state otherwise
                            ir::Transition::Unconditional(
                                if state + 1 == self.state {
                                    0
                                } else {
                                    state + 1
                                },
                            )
                        }
                    }
                };

                (vec![state_assign], transition_from_state, state_wire)
            })
            .multiunzip();

        if non_promoted_static_component {
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
                true_guard,
            );
            self.builder
                .add_continuous_assignments(vec![assign_fsm_start]);
        } else {
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

        self.builder.add_continuous_assignments(
            self.state2assigns
                .drain()
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

        // Instantiate the FSM with the assignments and transitions we built
        fsm.borrow_mut().assignments.extend(assignments);
        fsm.borrow_mut().transitions.extend(transitions);
        fsm
    }
}

impl Visitor for StaticFSMAllocation {
    fn start_static_control(
        &mut self,
        s: &mut calyx_ir::StaticControl,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        self.non_promoted_static_component = comp.is_static()
            && !(comp
                .attributes
                .has(ir::Attribute::Bool(ir::BoolAttr::Promoted)));
        let mut builder = ir::Builder::new(comp, sigs);

        let mut ssch = StaticSchedule::from(&mut builder);

        Ok(Action::change(ir::Control::fsm_enable(
            ssch.realize_fsm(s, self.non_promoted_static_component),
        )))
    }
    fn finish(
        &mut self,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> crate::traversal::VisResult {
        // If the component is static, get rid of all control components;
        // all assignments should already exist in the `wires` section
        if self.non_promoted_static_component {
            Ok(Action::Change(Box::new(ir::Control::empty())))
        } else {
            Ok(Action::Continue)
        }
    }
}
