use crate::analysis::{IncompleteTransition, StaticSchedule};
use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir::{self as ir, BoolAttr, GetAttributes};
use calyx_utils::CalyxResult;
use core::ops::Not;
use itertools::Itertools;

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

impl StaticSchedule<'_, '_> {
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
                    let final_state_guard =
                        self.leave_one_state_condition(guard, sen);

                    self.state += 1;
                    vec![IncompleteTransition::new(
                        self.state - 1,
                        final_state_guard,
                    )]
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
                (0..srep.num_repeats).fold(
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

        let (assignments, transitions, state2wires) =
            self.build_fsm_pieces(ir::RRC::clone(&fsm));

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
            let mut sch_constructor_false = StaticSchedule::from(&mut builder);
            let false_branch_fsm = sch_constructor_false.build_fsm(&s.fbranch);
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
                let thread_latency = thread.get_latency();
                let thread_fsm = sch_generator.build_fsm(thread);
                let mut trigger_thread = builder.build_assignment(
                    thread_fsm.borrow().get("start"),
                    signal_on.borrow().get("out"),
                    ir::Guard::True,
                );
                trigger_thread
                    .guard
                    .add_interval(ir::StaticTiming::new((0, thread_latency)));
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
