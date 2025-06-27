use crate::analysis::{IncompleteTransition, StaticSchedule};
use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir::{self as ir};
use calyx_utils::CalyxResult;

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

impl StaticSchedule<'_, '_> {
    /// Provided a static control node, calling this method on an empty `StaticSchedule`
    /// `sch` will build out the `latency` and `state2assigns` fields of `sch`, in
    /// preparation to replace the `StaticControl` node with an instance of `ir::FSM`.
    /// Every static assignment collected into `state2assigns` will have its existing guard
    /// "anded" with `guard`.
    fn build_abstract_fsm_with_loop(
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
                    let final_state_guard =
                        self.leave_one_state_condition(guard, sen);

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
                    // `build_abstract_fsm_with_loop` deal with registering the transition
                    // from the state(s) we just built.
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
                    self.build_abstract_fsm_with_loop(
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

        let (mut remaining_assignments, additional_looped_once_guard) = self
            .build_abstract_fsm_with_loop(
                control,
                ir::Guard::True,
                vec![],
                None,
            );

        // add loopback transitions to first state
        self.register_transitions(
            0,
            &mut remaining_assignments,
            ir::Guard::True,
        );

        let (mut assignments, transitions, state2wires) =
            self.build_fsm_pieces(ir::RRC::clone(&fsm));

        if non_promoted_static_component {
            // If the component is itself static<n>, there will be a super, parent
            // FSM that is expected to reset to state 0 exactly at cycle <n>
            // (the component is active during exactly [0..n-1]).
            //
            // We will get rid of the FSMEnable node from the
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
            // fsm[done] to maintain the dynamic interface. We'll do this in state 0
            // because the FSM controlling a static schedule cannot spend another
            // cycle in a separate DONE state (e.g. what if the static schedule is
            // invoked back-to-back in a repeat? The extra DONE state would push
            // the schedule one cycle back for every iteration.)

            // This register stores whether the FSM has been run exactly one time when
            // we return to state 0.
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
        fsm.borrow_mut().extend_fsm(assignments, transitions);
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
