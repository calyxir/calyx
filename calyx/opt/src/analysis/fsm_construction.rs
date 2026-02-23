use calyx_ir::{self as ir, build_assignments, guard};
use calyx_utils::math::bits_needed_for;
use core::ops::Not;
use itertools::Itertools;
use std::collections::HashMap;

type FSMPieces = (
    Vec<Vec<ir::Assignment<ir::Nothing>>>,
    Vec<ir::Transition>,
    Vec<ir::RRC<ir::Cell>>,
);

/// Represents an FSM transition that doesn't yet have a destination state.
#[derive(Clone)]
pub struct IncompleteTransition {
    source: u64,
    guard: ir::Guard<ir::Nothing>,
}

impl IncompleteTransition {
    pub fn new(source: u64, guard: ir::Guard<ir::Nothing>) -> Self {
        Self { source, guard }
    }
}

/// An instance of `StaticSchedule` is constrainted to live at least as long as
/// the component in which the static island that it represents lives.
pub struct StaticSchedule<'b, 'a: 'b> {
    /// Builder construct to add hardware to the component it's built from
    pub builder: &'b mut ir::Builder<'a>,
    /// Number of cycles to which the static schedule should count up
    pub state: u64,
    /// Maps every FSM state to assignments that should be active in that state
    pub state2assigns: HashMap<u64, Vec<ir::Assignment<ir::Nothing>>>,
    /// Parital map from FSM state to transitions out of that state.
    /// If a state has no mapping, assume it's an unconditional transition to
    /// state + 1.
    pub state2trans: HashMap<u64, ir::Transition>,
}

impl<'b, 'a> From<&'b mut ir::Builder<'a>> for StaticSchedule<'b, 'a> {
    fn from(builder: &'b mut ir::Builder<'a>) -> Self {
        StaticSchedule {
            builder,
            state: 0,
            state2assigns: HashMap::new(),
            state2trans: HashMap::new(),
        }
    }
}

impl StaticSchedule<'_, '_> {
    pub fn leave_one_state_condition(
        &mut self,
        guard: ir::Guard<ir::Nothing>,
        sen: &ir::StaticEnable,
    ) -> ir::Guard<ir::Nothing> {
        let signal_on = self.builder.add_constant(1, 1);
        let group_latency = sen.group.borrow().get_latency();

        // instantiate a local counter register
        let width = bits_needed_for(group_latency);
        let counter =
            self.builder
                .add_primitive("group_counter", "std_reg", &[width]);

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
                    &group_latency,
                );
                let mut assign = ir::Assignment::from(sassign);
                assign.and_guard(guard.clone());
                assign
            })
            .collect_vec();

        // guard reprsenting if counter is in final state
        let final_state_const =
            self.builder.add_constant(group_latency - 1, width);
        let final_state_wire: ir::RRC<ir::Cell> = self.builder.add_primitive(
            format!("const{}_{}_", group_latency - 1, width),
            "std_wire",
            &[width],
        );
        let final_state_guard = ir::Guard::CompOp(
            ir::PortComp::Eq,
            counter.borrow().get("out"),
            final_state_wire.borrow().get("out"),
        );
        let not_final_state_guard = final_state_guard.clone().not();

        // build assignments to increment / reset the counter
        let adder = self.builder.add_primitive("adder", "std_add", &[width]);
        let const_one = self.builder.add_constant(1, width);
        let const_zero = self.builder.add_constant(0, width);
        let incr_counter_assigns = build_assignments!(self.builder;
            final_state_wire["in"] = ? final_state_const["out"];
            adder["left"] = ? counter["out"];
            adder["right"] = ? const_one["out"];
            counter["write_en"] = ? signal_on["out"];
            counter["in"] = final_state_guard ? const_zero["out"];
            counter["in"] = not_final_state_guard ? adder["out"];
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

        final_state_guard
    }

    pub fn register_transitions(
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

    /// Builds counter logic and transitions for a repeat loop.
    ///
    /// Creates a counter register that tracks iterations, increment logic that runs
    /// on the last state of the loop body, and conditional transitions for looping back
    /// or exiting based on the counter value.
    ///
    /// # Arguments
    /// * `loop_start_state` - The first state of the loop body
    /// * `loop_end_state` - The last state of the loop body
    /// * `num_repeats` - Number of times to repeat the loop
    /// * `guard` - Guard condition for the loop
    ///
    /// # Returns
    /// The exit guard that signals when the loop is complete
    pub fn build_repeat_loop(
        &mut self,
        loop_start_state: u64,
        loop_end_state: u64,
        num_repeats: u64,
        guard: ir::Guard<ir::Nothing>,
    ) -> ir::Guard<ir::Nothing> {
        // Create a counter to track iterations
        let counter_width = bits_needed_for(num_repeats);
        let counter = self.builder.add_primitive(
            format!("repeat_counter_{loop_start_state}"),
            "std_reg",
            &[counter_width],
        );
        counter
            .borrow_mut()
            .add_attribute(ir::BoolAttr::FSMControl, 1);

        let signal_on = self.builder.add_constant(1, 1);

        // Final state constant: num_repeats - 1
        let counter_max =
            self.builder.add_constant(num_repeats - 1, counter_width);

        // Wire for comparing against final state
        let final_state_wire: ir::RRC<ir::Cell> = self.builder.add_primitive(
            format!("const{}_{}_", num_repeats - 1, counter_width),
            "std_wire",
            &[counter_width],
        );

        // Guard representing if counter is at final state
        let final_state_guard = ir::Guard::CompOp(
            ir::PortComp::Eq,
            counter.borrow().get("out"),
            final_state_wire.borrow().get("out"),
        );
        let not_final_state_guard = final_state_guard.clone().not();

        // Build increment logic
        let adder = self.builder.add_primitive(
            format!("repeat_adder_{loop_start_state}"),
            "std_add",
            &[counter_width],
        );
        adder
            .borrow_mut()
            .add_attribute(ir::BoolAttr::FSMControl, 1);

        let const_one = self.builder.add_constant(1, counter_width);
        let const_zero = self.builder.add_constant(0, counter_width);

        // Assignments to increment/reset the counter on the last state of the loop body
        let counter_assigns = build_assignments!(self.builder;
            final_state_wire["in"] = ? counter_max["out"];
            adder["left"] = ? counter["out"];
            adder["right"] = ? const_one["out"];
            counter["write_en"] = ? signal_on["out"];
            counter["in"] = final_state_guard ? const_zero["out"];
            counter["in"] = not_final_state_guard ? adder["out"];
        );

        // Add counter increment/reset assignments to the last state of the body
        self.state2assigns
            .entry(loop_end_state)
            .and_modify(|assigns| assigns.extend(counter_assigns.to_vec()))
            .or_insert(counter_assigns.to_vec());

        // Exit condition: counter == max
        let exit_guard = final_state_guard.clone();

        // Create the loop-back transition: if counter != max, go back to loop start
        let loop_back_guard = not_final_state_guard;
        let loop_back_transition =
            IncompleteTransition::new(loop_end_state, loop_back_guard);

        // Register the back edge
        self.register_transitions(
            loop_start_state,
            &mut vec![loop_back_transition],
            guard,
        );

        // Exit condition: counter == max
        exit_guard
    }

    pub fn build_fsm_pieces(&mut self, fsm: ir::RRC<ir::FSM>) -> FSMPieces {
        let signal_on = self.builder.add_constant(1, 1);
        (0..self.state)
            .map(|state| {
                // construct a wire to represent this state
                let state_wire: ir::RRC<ir::Cell> = self.builder.add_primitive(
                    format!("{}_{state}", fsm.borrow().name()),
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

                // merge `idle` and first `calc` state
                if state == 0 {
                    state_assign.and_guard(ir::guard!(fsm["start"]));
                }

                let transition_from_state = match self
                    .state2trans
                    .remove(&state)
                {
                    Some(mut transition) => {
                        // if in first state, transition conditioned on fsm[start]
                        let transition_mut_ref = &mut transition;
                        if state == 0 {
                            match transition_mut_ref {
                                ir::Transition::Unconditional(next_state) => {
                                    *transition_mut_ref =
                                        ir::Transition::Conditional(vec![
                                            (guard!(fsm["start"]), *next_state),
                                            (ir::Guard::True, 0),
                                        ]);
                                }
                                ir::Transition::Conditional(conditions) => {
                                    conditions.iter_mut().for_each(
                                        |(condition, _)| {
                                            condition.update(|g| {
                                                g.and(guard!(fsm["start"]))
                                            });
                                        },
                                    );
                                }
                            }
                        }

                        // add a default self-loop for every conditional transition
                        // if it doesn't already have it
                        if let ir::Transition::Conditional(trans) =
                            &mut transition
                            && !(trans.last_mut().unwrap().0.is_true())
                        {
                            trans.push((ir::Guard::True, state))
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

                (vec![state_assign], transition_from_state, state_wire)
            })
            .multiunzip()
    }
}
