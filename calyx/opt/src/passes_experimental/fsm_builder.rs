use crate::analysis::{IncompleteTransition, StaticSchedule};
use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir::{self as ir, GetAttributes};
use calyx_utils::CalyxResult;
const ACYCLIC: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::ACYCLIC);
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

impl StaticSchedule<'_, '_> {
    fn build_abstract(
        &mut self,
        scon: &ir::StaticControl,
        guard: ir::Guard<ir::Nothing>,
        mut transitions_to_curr: Vec<IncompleteTransition>,
    ) -> Vec<IncompleteTransition> {
        // allocate one state per cycle
        match scon {
            ir::StaticControl::Empty(_) => transitions_to_curr,
            ir::StaticControl::Enable(sen) => {
                if matches!(sen.get_attributes().get(ACYCLIC), Some(1)) {
                    // use parth's onestate code here
                    // for all parts of the FSM that want to transition to this enable,
                    // register their transitions in self.state2trans
                    self.register_transitions(
                        self.state,
                        &mut transitions_to_curr,
                        guard.clone(),
                    );

                    // one state for every
                    // cycle otherwise

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
                } else {
                    // must be cyclic otherwise
                    // use parth's onestate code here
                    // for all parts of the FSM that want to transition to this enable,
                    // register their transitions in self.state2trans
                    self.register_transitions(
                        self.state,
                        &mut transitions_to_curr,
                        guard.clone(),
                    );

                    // allocate one state if requested, and have one state for every
                    // cycle otherwise

                    let final_state_guard =
                        self.leave_one_state_condition(guard, sen);

                    self.state += 1;
                    vec![IncompleteTransition::new(
                        self.state - 1,
                        final_state_guard,
                    )]
                }
            }
            ir::StaticControl::Seq(sseq) => {
                todo!()
            }
            ir::StaticControl::Repeat(srep) => {
                todo!()
            }
            ir::StaticControl::If(sif) => {
                todo!()
            }
            ir::StaticControl::Par(spar) => {
                todo!()
            }
            ir::StaticControl::Invoke(_) => {
                unreachable!(
                    "`build_abstract_cyclic` encountered a `static_invoke` node. \
              Should have been compiled away."
                )
            }
        }
    }

    fn fsm_build(&mut self, control: &ir::StaticControl) -> ir::RRC<ir::FSM> {
        let fsm = self.builder.add_fsm("fsm");

        let mut remaining_assignments =
            self.build_abstract(control, ir::Guard::True, vec![]);

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

impl Visitor for FSMBuilder {
    fn enable(
        &mut self,
        sen: &mut calyx_ir::Enable,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        // let mut builder = ir::Builder::new(comp, sigs);
        // let signal_on = builder.add_constant(1, 1);

        // let mut sch_constructor = StaticSchedule::from(&mut builder);
        // if matches!(sen.get_attributes().get(ACYCLIC), Some(1)) {
        //      let fsm = sch_constructor.fsm_build(sen);
        // }
        Ok(Action::Continue)
    }

    fn finish_static_control(
        &mut self,
        scon: &mut calyx_ir::StaticControl,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        // non-promoted static components are static islands
        // pretend that only non-promoted so far.
        let mut builder = ir::Builder::new(comp, sigs);

        let mut ssch = StaticSchedule::from(&mut builder);

        Ok(Action::change(ir::Control::fsm_enable(
            ssch.fsm_build(scon),
        )))
        // otherwise need to do a dynamic handshake
        // Ok(Action::Continue)
    }
}
