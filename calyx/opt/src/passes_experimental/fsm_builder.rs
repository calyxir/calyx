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
        looped_once_guard: Option<ir::Guard<ir::Nothing>>,
    ) -> (Vec<IncompleteTransition>, Option<ir::Guard<ir::Nothing>>) {
        match scon {
            ir::StaticControl::Empty(_) => (transitions_to_curr, None),
            ir::StaticControl::Enable(sen) => {
                if matches!(sen.get_attributes().get(ACYCLIC), Some(1)) {
                    // allocate one state per cycle
                    println!("this is acyclic");
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
            ir::StaticControl::Seq(_sseq) => {
                todo!()
            }
            ir::StaticControl::Repeat(_srep) => {
                todo!()
            }
            ir::StaticControl::If(_sif) => {
                todo!()
            }
            ir::StaticControl::Par(_spar) => {
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
        // implementation for single static enable components for now.
        let mut builder = ir::Builder::new(comp, sigs);

        let mut ssch = StaticSchedule::from(&mut builder);

        Ok(Action::change(ir::Control::fsm_enable(
            ssch.fsm_build(scon),
        )))
    }
}
