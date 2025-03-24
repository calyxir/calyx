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

fn one_state_exists(scon: &ir::StaticControl) -> bool {
    match scon {
        ir::StaticControl::Empty(_) => false,
        ir::StaticControl::Enable(sen) => {
            sen.get_attributes().has(ir::BoolAttr::OneState)
        }
        ir::StaticControl::Seq(sseq) => sseq
            .stmts
            .iter()
            .fold(false, |exists, stmt| exists || (one_state_exists(stmt))),
        ir::StaticControl::If(sif) => {
            one_state_exists(&sif.tbranch) || one_state_exists(&sif.fbranch)
        }
        ir::StaticControl::Repeat(srep) => one_state_exists(&srep.body),
        ir::StaticControl::Invoke(_) | ir::StaticControl::Par(_) => {
            unreachable!()
        }
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
    ) -> Vec<IncompleteTransition> {
        match scon {
            ir::StaticControl::Empty(_) => transitions_to_curr,
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
                if sen.attributes.has(BoolAttr::OneState) {
                    let signal_on = self.builder.add_constant(1, 1);
                    let group_latency = sen.group.borrow().get_latency();

                    // instantiate a local counter register
                    let width = get_bit_width_from(group_latency);
                    let zero = self.builder.add_constant(0, 1);
                    let counter = self.builder.add_primitive(
                        "group_counter",
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
                            sassign.guard.replace_static_timing(
                                self.builder,
                                &counter,
                                &width,
                            );
                            let mut assign = ir::Assignment::from(sassign);
                            assign.and_guard(guard.clone());
                            assign
                        })
                        .collect_vec();

                    // make sure to actually enable the group at this state
                    let group = sen.group.clone();
                    let en_go: Vec<ir::Assignment<ir::Nothing>> =
                        build_assignments!(self.builder;
                            group["go"] = ? signal_on["out"];
                        )
                        .to_vec();

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
                    assigns.extend(en_go);

                    // push these assignments into the one state allocated for this
                    // enable
                    self.state2assigns
                        .entry(self.state)
                        .and_modify(|other_assigns| {
                            other_assigns.extend(assigns.clone());
                        })
                        .or_insert(assigns);

                    self.state += 1;
                    vec![IncompleteTransition::new(self.state - 1, final_state)]
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
                    vec![IncompleteTransition::new(
                        self.state - 1,
                        ir::Guard::True,
                    )]
                }
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

    /// Returns the FSM implementing the given control node, as well as the buidler
    /// object from which it was built.
    fn build_fsm(&mut self, control: &ir::StaticControl) -> ir::RRC<ir::FSM> {
        let fsm = self.builder.add_fsm("fsm");

        let mut remaining_assignments =
            self.build_abstract_fsm(control, ir::Guard::True, vec![]);

        // add loopback transitions to first state
        self.register_transitions(
            0,
            &mut remaining_assignments,
            ir::Guard::True,
        );
        let mut state2assigns: Vec<ir::RRC<ir::Cell>> = vec![];
        let (assignments, transitions): (
            Vec<Vec<ir::Assignment<ir::Nothing>>>,
            Vec<ir::Transition>,
        ) = (0..self.state)
            .map(|state| {
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
                        Some(mut transition) => {
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

                (assigns_at_state, transition_from_state)
            })
            .unzip();

        fsm.borrow_mut().extend_fsm(assignments, transitions);
        fsm
    }
}

impl Visitor for StaticRepeatFSMAllocation {
    fn finish_static_if(
        &mut self,
        s: &mut calyx_ir::StaticIf,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let signal_on = builder.add_constant(1, 1);

        // generate FSM for true branch
        let mut sch_constructor_true = StaticSchedule::from(&mut builder);
        let true_branch_fsm = sch_constructor_true.build_fsm(&s.tbranch);

        // generate FSM for false branch
        let mut sch_constructor_false = StaticSchedule::from(&mut builder);
        let false_branch_fsm = sch_constructor_false.build_fsm(&s.fbranch);

        // group to active each FSM conditionally
        let if_group = builder.add_static_group("if", s.latency);
        let true_guard: ir::Guard<ir::StaticTiming> =
            ir::Guard::port(ir::RRC::clone(&s.port));
        let false_guard = ir::Guard::not(true_guard.clone());

        // assignments to active each FSM
        let mut trigger_fsms = vec![
            builder.build_assignment(
                true_branch_fsm.borrow().get("start"),
                signal_on.borrow().get("out"),
                true_guard,
            ),
            builder.build_assignment(
                false_branch_fsm.borrow().get("start"),
                signal_on.borrow().get("out"),
                false_guard,
            ),
        ];

        // make sure [start] for each FSM is pulsed at most once, at the first
        // cycle
        trigger_fsms.iter_mut().for_each(|assign| {
            assign.guard.add_interval(ir::StaticTiming::new((0, 1)))
        });

        if_group.borrow_mut().assignments.extend(trigger_fsms);

        // ensure this group only gets one state in the parent FSM, and only
        // transitions out when the latency counter has completed
        let mut enable = ir::StaticControl::Enable(ir::StaticEnable {
            group: if_group,
            attributes: ir::Attributes::default(),
        });
        enable
            .get_mut_attributes()
            .insert(ir::BoolAttr::OneState, 1);

        Ok(Action::static_change(enable))
    }

    fn finish_static_par(
        &mut self,
        s: &mut calyx_ir::StaticPar,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let signal_on = builder.add_constant(1, 1);
        let par_group = builder.add_static_group("par", s.latency);
        par_group
            .borrow_mut()
            .assignments
            .extend(s.stmts.iter().map(|thread: &ir::StaticControl| {
                let mut sch_generator = StaticSchedule::from(&mut builder);
                let thread_fsm = sch_generator.build_fsm(thread);
                let mut trigger_thread = builder.build_assignment(
                    thread_fsm.borrow().get("start"),
                    signal_on.borrow().get("out"),
                    ir::Guard::True,
                );
                trigger_thread
                    .guard
                    .add_interval(ir::StaticTiming::new((0, 1)));
                trigger_thread
            }));

        let mut enable = ir::StaticControl::Enable(ir::StaticEnable {
            group: par_group,
            attributes: ir::Attributes::default(),
        });
        enable
            .get_mut_attributes()
            .insert(ir::BoolAttr::OneState, 1);

        Ok(Action::static_change(enable))
    }

    fn finish_static_repeat(
        &mut self,
        s: &mut calyx_ir::StaticRepeat,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let signal_on = builder.add_constant(1, 1);
        let repeat_group = builder.add_static_group("repeat", s.latency);
        let mut sch_generator = StaticSchedule::from(&mut builder);
        // let trigger_fsm = if !one_state_exists(&s.body) {
        let trigger_fsm = if false {
            // If there are no states that loop in place (i.e. that have registers
            // and adders to count latency), then we can unroll the repeat because
            // we won't then generate a lot of these resources.

            // Replace the static repeat node with a dummy node so we can create a
            // StaticControl instance to pass into `construct_schedule`
            let dummy_repeat = ir::StaticRepeat {
                attributes: ir::Attributes::default(),
                body: Box::new(ir::StaticControl::empty()),
                num_repeats: 0,
                latency: 0,
            };

            let repeat_node = std::mem::replace(s, dummy_repeat);
            let sc_wrapper = ir::StaticControl::Repeat(repeat_node);
            let fsm = sch_generator.build_fsm(&sc_wrapper);
            let mut trigger_thread = builder.build_assignment(
                fsm.borrow().get("start"),
                signal_on.borrow().get("out"),
                ir::Guard::True,
            );
            trigger_thread
                .guard
                .add_interval(ir::StaticTiming::new((0, 1)));
            trigger_thread
        } else {
            // This FSM implements the schedule for the body of the repeat
            let fsm = sch_generator.build_fsm(&s.body);

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
        enable
            .get_mut_attributes()
            .insert(ir::BoolAttr::OneState, 1);

        Ok(Action::static_change(enable))
    }
}
