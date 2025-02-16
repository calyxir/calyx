use crate::traversal::{Named, Visitor};
use calyx_ir::{self as ir, build_assignments, StaticTiming};
use core::ops::Not;
use std::collections::HashMap;
pub struct StaticFSMAllocation {}

impl Named for StaticFSMAllocation {
    fn name() -> &'static str {
        "static-fsm-alloc"
    }
    fn description() -> &'static str {
        "compiles a static schedule into an FSM construct"
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
    state2assigns: HashMap<u64, Vec<ir::Assignment<StaticTiming>>>,
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
        guard_opt: Option<ir::Guard<StaticTiming>>,
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
                            let mut sassign_copy = sassign.clone();
                            sassign_copy.and_guard(guard_opt.clone());
                            self.state2assigns
                                .entry(base + offset)
                                .and_modify(|other_assigns| {
                                    other_assigns.push(sassign_copy.clone())
                                })
                                .or_insert(vec![sassign_copy]);
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
                let build_branch_guard = |is_true_branch: bool| {
                    (match guard_opt.clone() {
                        None => ir::Guard::True,
                        Some(existing_guard) => existing_guard,
                    })
                    .and({
                        if is_true_branch {
                            ir::Guard::port(sif.port.clone())
                        } else {
                            ir::Guard::not(ir::Guard::port(sif.port.clone()))
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
    fn realize_fsm(&mut self) -> ir::RRC<ir::FSM> {
        // Declare the FSM
        let fsm = self.builder.add_fsm("fsm");

        // Construct the assignments and transitions that we'll eventually
        // put into the FSM declared above.
        let mut assignments = vec![Vec::new()];
        let mut transitions = vec![ir::Transition::Conditional(vec![
            (ir::guard!(fsm["start"]), 1),
            (ir::Guard::True, 0),
        ])];

        // Fill in the gaps for any missing state-mappings
        (0..self.latency).for_each(|state: u64| {
            self.state2assigns.entry(state).or_insert(Vec::new());
        });

        let (calc_state_transitions, calc_state_assignments): (
            Vec<ir::Transition>,
            Vec<Vec<ir::Assignment<ir::StaticTiming>>>,
        ) = self
            .state2assigns
            .drain()
            .map(|(state, assigns)| {
                (ir::Transition::new_uncond(state + 2), assigns)
            })
            .unzip();

        // insert transition from final calc state to `done` state
        let signal_on = self.builder.add_constant(1, 1);
        let true_guard = ir::Guard::True;
        assignments.push(
            build_assignments!(self.builder;
                fsm["done"] = true_guard ? signal_on["out"];
            )
            .to_vec(),
        );
        transitions.push(ir::Transition::Unconditional(0));

        // Instantiate the FSM with the assignments and transitions we built
        fsm.borrow_mut().assignments.extend(assignments);
        fsm.borrow_mut().transitions.extend(transitions);
        fsm
    }
}

impl Visitor for StaticFSMAllocation {}
