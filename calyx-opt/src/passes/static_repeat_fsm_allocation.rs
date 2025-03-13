use super::math_utilities::get_bit_width_from;
use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir::{self as ir, build_assignments, guard, BoolAttr, GetAttributes};
use calyx_utils::CalyxResult;
use core::ops::Not;
use itertools::Itertools;
use std::collections::HashMap;

pub struct StaticRepeatFSMAllocation {}

impl Named for StaticRepeatFSMAllocation {
    fn name() -> &'static str {
        "static-repeat-fsm-alloc"
    }
    fn description() -> &'static str {
        "compiles a static repeat into an FSM construct"
    }
}

impl ConstructVisitor for StaticRepeatFSMAllocation {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(StaticRepeatFSMAllocation {})
    }
    fn clear_data(&mut self) {}
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
struct StaticSchedule<'a> {
    /// Builder construct to add hardware to the component it's built from
    builder: ir::Builder<'a>,
    /// Number of cycles to which the static schedule should count up
    state: u64,
    /// Maps every FSM state to assignments that should be active in that state
    state2assigns: HashMap<u64, Vec<ir::Assignment<ir::Nothing>>>,
    /// Parital map from FSM state to transitions out of that state.
    /// If a state has no mapping, assume it's an unconditional transition to
    /// state + 1.
    state2trans: HashMap<u64, ir::Transition>,
}

impl<'a> From<ir::Builder<'a>> for StaticSchedule<'a> {
    fn from(builder: ir::Builder<'a>) -> Self {
        StaticSchedule {
            builder,
            state: 0,
            state2assigns: HashMap::new(),
            state2trans: HashMap::new(),
        }
    }
}

impl<'a> StaticSchedule<'a> {
    fn register_transitions(
        &mut self,
        transitions_to_curr: &mut Vec<IncompleteTransition>,
        and_guard: ir::Guard<ir::Nothing>,
    ) {
        transitions_to_curr.drain(..).for_each(
            |IncompleteTransition { source, guard }| {
                let complete_transition =
                    match (guard, &and_guard) {
                        (ir::Guard::True, ir::Guard::True) => {
                            ir::Transition::Unconditional(self.state)
                        }
                        (ir::Guard::True, _) => ir::Transition::Conditional(
                            vec![(and_guard.clone(), self.state)],
                        ),
                        (guard, ir::Guard::True) => {
                            ir::Transition::Conditional(vec![(
                                guard, self.state,
                            )])
                        }
                        (guard, and_guard) => ir::Transition::Conditional(
                            vec![(guard.and(and_guard.clone()), self.state)],
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
    ) -> Vec<IncompleteTransition> {
        match scon {
            ir::StaticControl::Empty(_) => transitions_to_curr,
            ir::StaticControl::Enable(sen) => {
                // for all parts of the FSM that want to transition to this enable,
                // register their transitions in self.state2trans
                self.register_transitions(
                    &mut transitions_to_curr,
                    guard.clone(),
                );

                // allocate one state if requested, and have one state for every
                // cycle otherwise
                if sen.attributes.has(BoolAttr::OneState) {
                    let signal_on = self.builder.add_constant(1, 1);
                    let group_latency = sen.group.borrow().get_latency();

                    // instantiate a local counter register
                    let width = get_bit_width_from(group_latency);
                    let zero = self.builder.add_constant(0, 1);
                    let counter = self.builder.add_primitive(
                        "repeat_counter",
                        "std_reg",
                        &[width],
                    );

                    // transform all assignments in the static group to read
                    // from the local counter
                    let mut assigns = sen
                        .group
                        .borrow_mut()
                        .assignments
                        .clone()
                        .drain(..)
                        .map(|mut sassign| {
                            sassign.guard.update(|static_guard| {
                                static_guard
                                    .compute_live_states(group_latency)
                                    .into_iter()
                                    // gauge state against register
                                    .map(|offset| {
                                        let state_const = self
                                            .builder
                                            .add_constant(offset, width);
                                        let g = ir::Guard::CompOp(
                                            ir::PortComp::Eq,
                                            counter.borrow().get("out"),
                                            state_const.borrow().get("out"),
                                        );
                                        g
                                    })
                                    // combine register reads with ||
                                    .fold(
                                        ir::Guard::port(
                                            zero.borrow().get("out"),
                                        ),
                                        ir::Guard::or,
                                    )
                            });
                            let mut assign = ir::Assignment::from(sassign);
                            assign.and_guard(guard.clone());
                            assign
                        })
                        .collect_vec();

                    // guard reprsenting if counter is in final state
                    let final_state = {
                        let final_state =
                            self.builder.add_constant(group_latency - 1, width);
                        let g = ir::Guard::CompOp(
                            ir::PortComp::Eq,
                            counter.borrow().get("out"),
                            final_state.borrow().get("out"),
                        );
                        g
                    };
                    let not_final_state = final_state.clone().not();

                    // build assignments to increment / reset the counter
                    let adder = self.builder.add_primitive(
                        "adder",
                        "std_add",
                        &[width],
                    );
                    let const_one = self.builder.add_constant(1, width);
                    let const_zero = self.builder.add_constant(0, width);
                    let incr_counter_assigns = build_assignments!(self.builder;
                        adder["left"] = ? counter["out"];
                        adder["right"] = ? const_one["out"];
                        counter["write_en"] = ? signal_on["out"];
                        counter["in"] = final_state ? const_zero["out"];
                        counter["in"] = not_final_state ? adder["out"];
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

                    self.state2trans.insert(
                        self.state,
                        ir::Transition::Conditional(vec![
                            (final_state, self.state + 1),
                            (ir::Guard::True, self.state),
                        ]),
                    );
                    self.state += 1;
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
                }
                // Don't know where to transition next; let the parent that called
                // `build_abstract_fsm` deal with registering the transition from the state(s)
                // we just built.
                vec![IncompleteTransition::new(self.state - 1, ir::Guard::True)]
            }
            ir::StaticControl::Seq(sseq) => sseq.stmts.iter().fold(
                transitions_to_curr,
                |transitions_to_this_stmt, stmt| {
                    self.build_abstract_fsm(
                        stmt,
                        guard.clone(),
                        transitions_to_this_stmt,
                    )
                },
            ),

            ir::StaticControl::If(sif) => {
                // construct a guard on the static assignments in the each branch
                let build_branch_guard =
                    |is_true_branch: bool| -> ir::Guard<ir::Nothing> {
                        guard.clone().and({
                            if is_true_branch {
                                ir::Guard::port(sif.port.clone())
                            } else {
                                ir::Guard::not(ir::Guard::port(
                                    sif.port.clone(),
                                ))
                            }
                        })
                    };

                self.build_abstract_fsm(
                    &sif.tbranch,
                    guard.clone().and(build_branch_guard(true)),
                    transitions_to_curr.clone(),
                )
                .into_iter()
                .chain(self.build_abstract_fsm(
                    &sif.fbranch,
                    guard.clone().and(build_branch_guard(false)),
                    transitions_to_curr.clone(),
                ))
                .collect()
            }
            ir::StaticControl::Repeat(srep) => {
                // unroll an encountered repeat loop. usually these are compiled away
                (0..srep.num_repeats).into_iter().fold(
                    transitions_to_curr,
                    |transitions_to_this_body, _| {
                        self.build_abstract_fsm(
                            &srep.body,
                            guard.clone(),
                            transitions_to_this_body,
                        )
                    },
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

    /// Provided a static control node, calling this method on an empty `StaticSchedule`
    /// `sch` will build out the `latency` and `state2assigns` fields of `sch`, in
    /// preparation to replace the `StaticControl` node with an instance of `ir::FSM`.
    /// Every static assignment collected into `state2assigns` will have its existing guard
    /// "anded" with `guard`.
    fn construct_schedule(
        &mut self,
        scon: &ir::StaticControl,
        guard: ir::Guard<ir::Nothing>,
    ) {
        match scon {
            ir::StaticControl::Empty(_) => (),
            ir::StaticControl::Enable(sen) => {
                if sen.attributes.has(BoolAttr::OneState) {
                    let signal_on = self.builder.add_constant(1, 1);
                    let group_latency = sen.group.borrow().get_latency();

                    // instantiate a local counter register
                    let width = get_bit_width_from(group_latency);
                    let zero = self.builder.add_constant(0, 1);
                    let counter = self.builder.add_primitive(
                        "repeat_counter",
                        "std_reg",
                        &[width],
                    );

                    // transform all assignments in the static group to read
                    // from the local counter
                    let mut assigns = sen
                        .group
                        .borrow_mut()
                        .assignments
                        .clone()
                        .drain(..)
                        .map(|mut sassign| {
                            sassign.guard.update(|static_guard| {
                                static_guard
                                    .compute_live_states(group_latency)
                                    .into_iter()
                                    // gauge state against register
                                    .map(|offset| {
                                        let state_const = self
                                            .builder
                                            .add_constant(offset, width);
                                        let g = ir::Guard::CompOp(
                                            ir::PortComp::Eq,
                                            counter.borrow().get("out"),
                                            state_const.borrow().get("out"),
                                        );
                                        g
                                    })
                                    // combine register reads with ||
                                    .fold(
                                        ir::Guard::port(
                                            zero.borrow().get("out"),
                                        ),
                                        ir::Guard::or,
                                    )
                            });
                            let mut assign = ir::Assignment::from(sassign);
                            assign.and_guard(guard.clone());
                            assign
                        })
                        .collect_vec();

                    // guard reprsenting if counter is in final state
                    let final_state = {
                        let final_state =
                            self.builder.add_constant(group_latency - 1, width);
                        let g = ir::Guard::CompOp(
                            ir::PortComp::Eq,
                            counter.borrow().get("out"),
                            final_state.borrow().get("out"),
                        );
                        g
                    };
                    let not_final_state = final_state.clone().not();

                    // build assignments to increment / reset the counter
                    let adder = self.builder.add_primitive(
                        "adder",
                        "std_add",
                        &[width],
                    );
                    let const_one = self.builder.add_constant(1, width);
                    let const_zero = self.builder.add_constant(0, width);
                    let incr_counter_assigns = build_assignments!(self.builder;
                        adder["left"] = ? counter["out"];
                        adder["right"] = ? const_one["out"];
                        counter["write_en"] = ? signal_on["out"];
                        counter["in"] = final_state ? const_zero["out"];
                        counter["in"] = not_final_state ? adder["out"];
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

                    self.state2trans.insert(
                        self.state,
                        ir::Transition::Conditional(vec![
                            (final_state, self.state + 1),
                            (ir::Guard::True, self.state),
                        ]),
                    );

                    self.state += 1;
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
                }
            }
            ir::StaticControl::Seq(sseq) => {
                sseq.stmts.iter().for_each(|stmt| {
                    self.construct_schedule(stmt, guard.clone());
                });
            }

            ir::StaticControl::If(sif) => {
                // construct a guard on the static assignments in the each branch
                let build_branch_guard =
                    |is_true_branch: bool| -> ir::Guard<ir::Nothing> {
                        guard.clone().and({
                            if is_true_branch {
                                ir::Guard::port(sif.port.clone())
                            } else {
                                ir::Guard::not(ir::Guard::port(
                                    sif.port.clone(),
                                ))
                            }
                        })
                    };
                // Construct the schedule based on the true branch.
                // Since this construction will progress the schedule's latency,
                // we need to bring the baseline back to its original value before
                // doing the same for the false branch.
                self.construct_schedule(&sif.tbranch, build_branch_guard(true));
                self.state -= sif.tbranch.get_latency();
                self.construct_schedule(
                    &sif.fbranch,
                    build_branch_guard(false),
                );
                self.state -= sif.fbranch.get_latency();
                // Finally, just progress the latency by the maximum of the
                // branches' latencies
                self.state += sif.latency;
            }
            ir::StaticControl::Par(spar) => {
                // for each par thread, construct the schedule and reset
                // the baseline latency to correctly compile the next par thread
                spar.stmts.iter().for_each(|stmt| {
                    self.construct_schedule(stmt, guard.clone());
                    self.state -= stmt.get_latency();
                });
                self.state += spar.latency;
            }
            ir::StaticControl::Repeat(srep) => {
                // unroll an encountered repeat loop
                for _ in 0..srep.num_repeats {
                    self.construct_schedule(&srep.body, guard.clone());
                }
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
    fn realize_static_repeat_fsm(&mut self, fsm: &mut ir::RRC<ir::FSM>) {
        let (assignments, transitions): (
            Vec<Vec<ir::Assignment<ir::Nothing>>>,
            Vec<ir::Transition>,
        ) = (0..self.state)
            .map(|state: u64| {
                let assigns_at_state = match self.state2assigns.remove(&state) {
                    None => vec![],
                    Some(mut assigns) => {
                        // merge `idle` and first `calc` state
                        if state == 0 {
                            assigns.iter_mut().for_each(|assign| {
                                assign.and_guard(guard!(fsm["start"]));
                            });
                        }
                        assigns
                    }
                };

                let transition_from_state =
                    match self.state2trans.remove(&state) {
                        Some(transition) => transition,
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

                (assigns_at_state, transition_from_state)
            })
            .unzip();

        // Instantiate the FSM with the assignments and transitions we built
        fsm.borrow_mut().extend_fsm(assignments, transitions);
    }
}

impl Visitor for StaticRepeatFSMAllocation {
    fn finish_static_repeat(
        &mut self,
        s: &mut calyx_ir::StaticRepeat,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let replacement_group = builder.add_static_group("repeat", s.latency);

        // trigger fsm[start] dependent on the [go] of the group that will
        // replace this StaticRepeat node
        let mut fsm = builder.add_fsm("fsm");
        let mut start_fsm = builder.build_assignment(
            fsm.borrow().get("start"),
            replacement_group.borrow().get("go"),
            ir::Guard::True,
        );

        // replace the static repeat node with a dummy node so we can actually own
        // the &mut StaticRepeat this function provides; we need it to build a
        // StaticControl instance to pass into `construct_schedule`
        let dummy_repeat = ir::StaticRepeat {
            attributes: ir::Attributes::default(),
            body: Box::new(ir::StaticControl::empty()),
            num_repeats: 0,
            latency: 0,
        };
        let repeat_node = std::mem::replace(s, dummy_repeat);
        let sc_wrapper = ir::StaticControl::Repeat(repeat_node);
        let mut ssch = StaticSchedule::from(builder);

        // build out the above-constructed fsm for the schedule
        ssch.construct_schedule(&sc_wrapper, ir::Guard::True);
        ssch.realize_static_repeat_fsm(&mut fsm);

        // pulse fsm[start] at first cycle of this group
        start_fsm.guard.add_interval(ir::StaticTiming::new((0, 1)));
        replacement_group.borrow_mut().assignments.push(start_fsm);

        // make sure only one state gets allocated for this group (i.e. avoid
        // inlining the entire latency into the FSM)
        let mut static_en = ir::StaticEnable {
            group: replacement_group,
            attributes: ir::Attributes::default(),
        };

        static_en
            .get_mut_attributes()
            .insert(ir::BoolAttr::OneState, 1);

        Ok(Action::static_change(ir::StaticControl::Enable(static_en)))
    }
}
