use crate::analysis::{IncompleteTransition, StaticSchedule};
use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir::{self as ir, GetAttributes};
use calyx_utils::CalyxResult;
const ACYCLIC: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::ACYCLIC);
const UNROLL: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::UNROLL);
const OFFLOAD: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::OFFLOAD);
const INLINE: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::INLINE);

pub struct FSMBuilder {}

impl Named for FSMBuilder {
    fn name() -> &'static str {
        "fsm-builder"
    }
    fn description() -> &'static str {
        "generates medium fsms in one pass for static and dynamic"
    }
}

impl ConstructVisitor for FSMBuilder {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(FSMBuilder {})
    }
    fn clear_data(&mut self) {}
}

// node that passes into this function is a part of the schedule
impl StaticSchedule<'_, '_> {
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
                if matches!(sen.get_attributes().get(ACYCLIC), Some(1)) {
                    // allocate one state per cycle
                    // for all parts of the FSM that want to transition to this enable,
                    // register their transitions in self.state2trans
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
                } else {
                    // must be cyclic otherwise
                    // placeholder code for now.
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
                }
            }
            ir::StaticControl::Seq(sseq) => (
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
            ),
            ir::StaticControl::Repeat(srep) => {
                if matches!(srep.get_attributes().get(UNROLL), Some(1)) {
                    // unroll an encountered repeat loop. usually these are compiled away
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
                } else if matches!(srep.get_attributes().get(OFFLOAD), Some(1))
                {
                    todo!()
                } else if matches!(srep.get_attributes().get(INLINE), Some(1)) {
                    // do nothing, let the function that called this
                    todo!()
                } else {
                    todo!()
                }
            }
            ir::StaticControl::If(_sif) => {
                todo!()
            }
            ir::StaticControl::Par(_spar) => {
                todo!()
            }
            ir::StaticControl::Invoke(_) => {
                unreachable!(
                    "`build_abstract` encountered a `static_invoke` node. \
              Should have been compiled away."
                )
            }
        }
    }

    fn fsm_build(
        &mut self,
        control: &ir::StaticControl,
        build_kind: Option<bool>, // need to get better type name. Some(True) means non-promoted-static-component. False means promoted/static island. Otherwise it's a
    ) -> ir::RRC<ir::FSM> {
        let true_guard = ir::Guard::True;
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

        // case work starts here
        match build_kind {
            Some(true) => {
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
            }
            Some(false) => {
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
            None => {
                // Do nothing because we want to build a normal kind of component?
            }
        }
        // build up the fsm here and return

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

impl Visitor for FSMBuilder {
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
        // implementation for single static enable components for now.
        let mut builder = ir::Builder::new(comp, sigs);

        let mut ssch = StaticSchedule::from(&mut builder);

        Ok(Action::change(ir::Control::fsm_enable(
            ssch.fsm_build(scon, Some(non_promoted_static_component)),
        )))
    }
}
