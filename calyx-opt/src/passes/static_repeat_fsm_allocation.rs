use super::math_utilities::get_bit_width_from;
use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
use calyx_ir::{self as ir, guard, BoolAttr, GetAttributes};
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
            ir::StaticControl::Empty(_) | ir::StaticControl::Invoke(_) => (),
            ir::StaticControl::Enable(sen) => {
                if sen.attributes.has(BoolAttr::OneState) {
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
                    let assigns = sen
                        .group
                        .borrow_mut()
                        .assignments
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
                                            state_const.borrow().get("out"),
                                            counter.borrow().get("out"),
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

                    // push these assignments into the one state allocated for this
                    // enable
                    self.state2assigns
                        .entry(self.state)
                        .and_modify(|other_assigns| {
                            other_assigns.extend(assigns.clone())
                        })
                        .or_insert(assigns);

                    // build transition out of this state
                    let final_state_guard = {
                        let state_const =
                            self.builder.add_constant(self.state, width);
                        let g = ir::Guard::CompOp(
                            ir::PortComp::Eq,
                            state_const.borrow().get("out"),
                            counter.borrow().get("out"),
                        );
                        g
                    };

                    self.state2trans.insert(
                        self.state,
                        ir::Transition::Conditional(vec![
                            (final_state_guard, self.state + 1),
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
                }

                self.state += sen.group.borrow().latency;
            }
            ir::StaticControl::Seq(sseq) => {
                sseq.stmts.iter().for_each(|stmt| {
                    self.construct_schedule(stmt, guard.clone());
                });
            }
            ir::StaticControl::Repeat(srep) => {
                for _ in 0..srep.num_repeats {
                    self.construct_schedule(&srep.body, guard.clone());
                }
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
        }
    }

    /// Given a filled-out static schedule, construct an FSM based on the state mappings
    /// in `state2assigns`.
    fn realize_static_repeat_fsm(&mut self) -> ir::RRC<ir::FSM> {
        let true_guard = ir::Guard::True;

        // Declare the FSM
        let fsm = self.builder.add_fsm("fsm");

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
                                    (true_guard.clone(), 0),
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
        fsm
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

        // replace the static repeat node with a dummy node so we can create a
        // StaticControl instance to pass into `construct_schedule`
        let dummy_repeat = ir::StaticRepeat {
            attributes: ir::Attributes::default(),
            body: Box::new(ir::StaticControl::empty()),
            num_repeats: 0,
            latency: 0,
        };

        let repeat_node = std::mem::replace(s, dummy_repeat);
        let sc_wrapper = ir::StaticControl::Repeat(repeat_node);
        let builder = ir::Builder::new(comp, sigs);
        let mut ssch = StaticSchedule::from(builder);

        // generate an fsm for the schedule
        ssch.construct_schedule(&sc_wrapper, ir::Guard::True);
        let fsm = ssch.realize_static_repeat_fsm();

        // trigger fsm[start] dependent on the [go] of the group that will
        // replace this StaticRepeat node
        let mut start_fsm = ir::Assignment::new(
            fsm.borrow().get("start"),
            replacement_group.borrow().get("go"),
        );
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
