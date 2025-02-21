use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir::{self as ir, build_assignments, guard};
use calyx_utils::CalyxResult;
use core::ops::Not;
use itertools::MultiUnzip;
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

/// An instance of `StaticSchedule` is constrainted to live at least as long as
/// the component in which the static island that it represents lives.
struct StaticSchedule<'a> {
    /// Builder construct to add hardware to the component it's built from
    builder: ir::Builder<'a>,
    /// Number of cycles to which the static schedule should count up
    latency: u64,
    /// Maps every FSM state to assignments that should be active in that state
    state2assigns: HashMap<u64, Vec<ir::Assignment<ir::Nothing>>>,
}

impl<'a> From<ir::Builder<'a>> for StaticSchedule<'a> {
    fn from(builder: ir::Builder<'a>) -> Self {
        StaticSchedule {
            builder,
            latency: 0,
            state2assigns: HashMap::new(),
        }
    }
}

impl<'a> StaticSchedule<'a> {
    /// Provided a static control node, calling this method on an empty `StaticSchedule`
    /// `sch` will build out the `latency` and `state2assigns` fields of `sch`, in
    /// preparation to replace the `StaticControl` node with an instance of `ir::FSM`.
    /// If the argument `guard_opt` is `Some(guard)`, then every static assignment
    /// collected into `state2assigns` will have its existing guard "anded" with `guard`.
    /// If it is `None`, then the assignment remains as is.
    fn construct_schedule(
        &mut self,
        scon: &ir::StaticControl,
        guard_opt: Option<ir::Guard<ir::Nothing>>,
    ) {
        match scon {
            ir::StaticControl::Empty(_) | ir::StaticControl::Invoke(_) => (),
            ir::StaticControl::Enable(sen) => {
                let base = self.latency;
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
                            assign.and_guard(guard_opt.clone());
                            // add this assignment to the list of assignments
                            // that are supposed to be valid at this state
                            self.state2assigns
                                .entry(base + offset)
                                .and_modify(|other_assigns| {
                                    other_assigns.push(assign.clone())
                                })
                                .or_insert(vec![assign]);
                        })
                });
                self.latency += sen.group.borrow().latency;
            }
            ir::StaticControl::Seq(sseq) => {
                sseq.stmts.iter().for_each(|stmt| {
                    self.construct_schedule(stmt, guard_opt.clone());
                });
            }
            ir::StaticControl::Repeat(srep) => {
                for _ in 0..srep.num_repeats {
                    self.construct_schedule(&srep.body, guard_opt.clone());
                }
            }
            ir::StaticControl::If(sif) => {
                // construct a guard on the static assignments in the each branch
                let build_branch_guard =
                    |is_true_branch: bool| -> ir::Guard<ir::Nothing> {
                        (match guard_opt.clone() {
                            None => ir::Guard::True,
                            Some(existing_guard) => existing_guard,
                        })
                        .and({
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
                self.construct_schedule(
                    &sif.tbranch,
                    Some(build_branch_guard(true)),
                );
                self.latency -= sif.tbranch.get_latency();
                self.construct_schedule(
                    &sif.fbranch,
                    Some(build_branch_guard(false)),
                );
                self.latency -= sif.fbranch.get_latency();
                // Finally, just progress the latency by the maximum of the
                // branches' latencies
                self.latency += sif.latency;
            }
            ir::StaticControl::Par(spar) => {
                // for each par thread, construct the schedule and reset
                // the baseline latency to correctly compile the next par thread
                spar.stmts.iter().for_each(|stmt| {
                    self.construct_schedule(stmt, guard_opt.clone());
                    self.latency -= stmt.get_latency();
                });
                self.latency += spar.latency;
            }
        }
    }

    /// Given a filled-out static schedule, construct an FSM based on the state mappings
    /// in `state2assigns`.
    fn realize_fsm(
        &mut self,
        non_promoted_static_component: bool,
    ) -> ir::RRC<ir::FSM> {
        let true_guard = ir::Guard::True;
        let signal_on = self.builder.add_constant(1, 1);

        // Declare the FSM
        let fsm = self.builder.add_fsm("fsm");

        // Fill in the FSM construct to contain unconditional transitions from n
        // to n+1 at each cycle (except for loopback at final state), and to
        // hold the corresponding state-wire high at the right cycle.
        let (mut assignments, mut transitions, state2wires): (
            Vec<Vec<ir::Assignment<ir::Nothing>>>,
            Vec<ir::Transition>,
            Vec<ir::RRC<ir::Cell>>,
        ) = (0..self.latency)
            .map(|state: u64| {
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

                let transition = if state == 0 {
                    // merge first "calc" state with first "idle" state
                    state_assign.and_guard(Some(ir::guard!(fsm["start"])));

                    // set transition out of first state, which is conditional on reading fsm[start]
                    ir::Transition::Conditional(vec![
                        (guard!(fsm["start"]), 1 % self.latency),
                        (true_guard.clone(), 0),
                    ])
                } else {
                    // loopback to start at final state, and increment state otherwise
                    ir::Transition::Unconditional(
                        if state + 1 == self.latency {
                            0
                        } else {
                            state + 1
                        },
                    )
                };

                (vec![state_assign], transition, state_wire)
            })
            .multiunzip();

        if non_promoted_static_component {
            // If the component is fully static, there will be exactly one
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
            // If the component is not static (i.e. this static schedule is just
            // a static island), then we need to make sure that fsm[done] is
            // assigned to. There are almost certainly other parts of this component's
            // schedule that require the [done] signal out of a static-island FSM.
            match transitions.last_mut().unwrap() {
                // if latency > 1, you'll have an unconditional transition to 0;
                // just change this to point to the new final state
                ir::Transition::Unconditional(state) => {
                    *state = self.latency;
                }
                // if latency = 1, you'll have a conditional transition back to 0;
                // also change this to point to the final state
                ir::Transition::Conditional(guarded_state) => {
                    guarded_state.first_mut().unwrap().1 += 1;
                }
            }

            // place done condition assignment and transition back to 0
            assignments.push(
                build_assignments!(self.builder;
                    fsm["done"] = true_guard ? signal_on["out"];
                )
                .to_vec(),
            );
            transitions.push(ir::Transition::Unconditional(0));
        }

        // Transform all the state-dependent assignments within the static schedule
        // into continuous assignments, guarded by the value of their corresponding
        // state wires (constructed above)
        self.builder.add_continuous_assignments(
            self.state2assigns
                .drain()
                .flat_map(|(state, mut assigns)| {
                    assigns.iter_mut().for_each(|assign| {
                        assign.and_guard(Some(ir::Guard::port(
                            state2wires
                                .get(state as usize)
                                .unwrap()
                                .borrow()
                                .get("out"),
                        )));
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
        let mut ssch = StaticSchedule::from(ir::Builder::new(comp, sigs));
        ssch.construct_schedule(s, None);
        Ok(Action::change(ir::Control::fsm_enable(
            ssch.realize_fsm(self.non_promoted_static_component),
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
