use super::math_utilities::get_bit_width_from;
use crate::errors::CalyxResult;
use crate::{build_assignments, guard, passes, structure};
use crate::{
    errors::Error,
    ir::{
        self,
        traversal::{Action, Named, VisResult, Visitor},
        LibrarySignatures, RRC,
    },
};
use ir::IRPrinter;
use itertools::Itertools;
use petgraph::graph::DiGraph;
use std::collections::HashMap;
use std::rc::Rc;

/// Represents the execution schedule of a control program.
#[derive(Default)]
struct Schedule {
    /// Assigments that should be enabled in a given state.
    pub enables: HashMap<u64, Vec<ir::Assignment>>,
    /// Transition from one state to another when the guard is true.
    pub transitions: Vec<(u64, u64, ir::Guard)>,
}

impl Schedule {
    /// Validate that all states are reachable in the transition graph.
    fn validate(&self) {
        let graph = DiGraph::<(), u32>::from_edges(
            &self
                .transitions
                .iter()
                .map(|(s, e, _)| (*s as u32, *e as u32))
                .collect::<Vec<_>>(),
        );

        debug_assert!(
            petgraph::algo::connected_components(&graph) == 1,
            "State transition graph has unreachable states (graph has more than one connected component).");
    }

    /// Return the max state in the transition graph
    fn last_state(&self) -> u64 {
        self.transitions
            .iter()
            .max_by_key(|(_, s, _)| s)
            .expect("Schedule::transition is empty!")
            .1
    }

    /// Print out the current schedule
    #[allow(dead_code)]
    fn display(&self) {
        self.enables
            .iter()
            .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .for_each(|(state, assigns)| {
                eprintln!("======== {} =========", state);
                assigns.iter().for_each(|assign| {
                    IRPrinter::write_assignment(
                        assign,
                        0,
                        &mut std::io::stderr(),
                    )
                    .expect("Printing failed!");
                    eprintln!();
                })
            });
        eprintln!("------------");
        self.transitions
            .iter()
            .sorted_by(|(k1, _, _), (k2, _, _)| k1.cmp(k2))
            .for_each(|(i, f, g)| {
                eprintln!("({}, {}): {}", i, f, IRPrinter::guard_str(g));
            })
    }

    /// Implement a given [Schedule] and return the name of the [ir::Group] that
    /// implements it.
    fn realize_schedule(self, builder: &mut ir::Builder) -> RRC<ir::Group> {
        self.validate();
        let final_state = self.last_state();
        let fsm_size = get_bit_width_from(
            final_state + 1, /* represent 0..final_state */
        );
        structure!(builder;
            let fsm = prim std_reg(fsm_size);
            let signal_on = constant(1, 1);
            let last_state = constant(final_state, fsm_size);
            let first_state = constant(0, fsm_size);
        );

        // The compilation group
        let group = builder.add_group("tdcc");

        // Enable assignments
        group.borrow_mut().assignments.extend(
            self.enables
                .into_iter()
                .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
                .flat_map(|(state, mut assigns)| {
                    let state_const = builder.add_constant(state, fsm_size);
                    let state_guard =
                        guard!(fsm["out"]).eq(guard!(state_const["out"]));
                    assigns.iter_mut().for_each(|asgn| {
                        asgn.guard.update(|g| g.and(state_guard.clone()))
                    });
                    assigns
                }),
        );

        // Transition assignments
        group.borrow_mut().assignments.extend(
            self.transitions.into_iter().flat_map(|(s, e, guard)| {
                structure!(builder;
                    let end_const = constant(e, fsm_size);
                    let start_const = constant(s, fsm_size);
                );
                let ec_borrow = end_const.borrow();
                let trans_guard =
                    guard!(fsm["out"]).eq(guard!(start_const["out"])) & guard;

                vec![
                    builder.build_assignment(
                        fsm.borrow().get("in"),
                        ec_borrow.get("out"),
                        trans_guard.clone(),
                    ),
                    builder.build_assignment(
                        fsm.borrow().get("write_en"),
                        signal_on.borrow().get("out"),
                        trans_guard,
                    ),
                ]
            }),
        );

        // Done condition for group
        let last_guard = guard!(fsm["out"]).eq(guard!(last_state["out"]));
        let done_assign = builder.build_assignment(
            group.borrow().get("done"),
            signal_on.borrow().get("out"),
            last_guard.clone(),
        );
        group.borrow_mut().assignments.push(done_assign);

        // Cleanup: Add a transition from last state to the first state.
        let mut reset_fsm = build_assignments!(builder;
            fsm["in"] = last_guard ? first_state["out"];
            fsm["write_en"] = last_guard ? signal_on["out"];
        );
        builder
            .component
            .continuous_assignments
            .append(&mut reset_fsm);

        group
    }
}

/// Computes the entry and exit points of a given [ir::Control] program.
///
/// ## Example
/// In the following Calyx program:
/// ```
/// while comb_reg.out {
///   seq {
///     incr;
///     cond0;
///   }
/// }
/// ```
/// The entry point is `incr` and the exit point is `cond0`.
///
/// Multiple entry and exit points are created when conditions are used:
/// ```
/// while comb_reg.out {
///   incr;
///   if comb_reg2.out {
///     true;
///   } else {
///     false;
///   }
/// }
/// ```
/// The entry set is `incr` while exit set is `[true, false]`.
fn entries_and_exits(
    con: &ir::Control,
    cur_state: u64,
    is_entry: bool,
    is_exit: bool,
    entries: &mut Vec<u64>,
    exits: &mut Vec<u64>,
) -> u64 {
    match con {
        ir::Control::Enable(_) => {
            if is_entry {
                entries.push(cur_state)
            } else if is_exit {
                exits.push(cur_state)
            }
            cur_state + 1
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            let len = stmts.len();
            let mut cur = cur_state;
            for (idx, stmt) in stmts.iter().enumerate() {
                let entry = idx == 0 && is_entry;
                let exit = idx == len - 1 && is_exit;
                cur = entries_and_exits(stmt, cur, entry, exit, entries, exits);
            }
            cur
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            let tru_nxt = entries_and_exits(
                tbranch, cur_state, is_entry, is_exit, entries, exits,
            );
            entries_and_exits(
                fbranch, tru_nxt, is_entry, is_exit, entries, exits,
            )
        }
        ir::Control::While(ir::While { body, .. }) => entries_and_exits(
            body, cur_state, is_entry, is_exit, entries, exits,
        ),
        ir::Control::Invoke(_) => todo!(),
        ir::Control::Empty(_) => todo!(),
        ir::Control::Par(_) => todo!(),
    }
}

/// Calculate [ir::Assignment] to enable in each FSM state and [ir::Guard] required to transition
/// between FSM states. Each step of the calculation computes the previous states that still need
/// to transition.
fn calculate_states_recur(
    con: &ir::Control,
    // The current state
    cur_state: u64,
    // The set of previous states that want to transition into cur_state
    prev_states: Vec<(u64, ir::Guard)>,
    // Guard that needs to be true to reach this state
    with_guard: &ir::Guard,
    // Current schedule.
    schedule: &mut Schedule,
    // Component builder
    builder: &mut ir::Builder,
) -> CalyxResult<(Vec<(u64, ir::Guard)>, u64)> {
    match con {
        // Enable the group in `cur_state` and construct all transitions from
        // previous states to this enable.
        ir::Control::Enable(ir::Enable { group, .. }) => {
            let not_done = !guard!(group["done"]);
            let signal_on = builder.add_constant(1, 1);
            let mut en_go = build_assignments!(builder;
                group["go"] = not_done ? signal_on["out"];
            );
            schedule
                .enables
                .entry(cur_state)
                .or_default()
                .append(&mut en_go);

            let transitions = prev_states
                .into_iter()
                .map(|(st, guard)| (st, cur_state, guard & with_guard.clone()));
            schedule.transitions.extend(transitions);

            let done_cond = guard!(group["done"]);
            let nxt = cur_state + 1;
            Ok((vec![(cur_state, done_cond)], nxt))
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            let mut prev = prev_states;
            let mut cur = cur_state;
            for (idx, stmt) in stmts.iter().enumerate() {
                let res = calculate_states_recur(
                    stmt,
                    cur,
                    prev,
                    // Only the first group gets the with_guard for the previous state.
                    if idx == 0 {
                        with_guard
                    } else {
                        &ir::Guard::True
                    },
                    schedule,
                    builder,
                )?;
                prev = res.0;
                cur = res.1;
            }
            Ok((prev, cur))
        }
        ir::Control::If(ir::If {
            port,
            cond,
            tbranch,
            fbranch,
            ..
        }) => {
            if cond.is_some() {
                return Err(Error::MalformedStructure(format!("{}: Found group `{}` in with position of if. This should have compiled away.", TopDownCompileControl::name(), cond.as_ref().unwrap().borrow().name())));
            }
            let port_guard = ir::Guard::port(Rc::clone(port));
            // Add transitions for the true branch
            let (tru_prev, tru_nxt) = calculate_states_recur(
                tbranch,
                cur_state,
                prev_states.clone(),
                &port_guard,
                schedule,
                builder,
            )?;
            let (fal_prev, fal_nxt) = calculate_states_recur(
                fbranch,
                tru_nxt,
                prev_states,
                &!port_guard,
                schedule,
                builder,
            )?;
            let prevs =
                tru_prev.into_iter().chain(fal_prev.into_iter()).collect();
            Ok((prevs, fal_nxt))
        }
        ir::Control::While(ir::While {
            cond, port, body, ..
        }) => {
            if cond.is_some() {
                return Err(Error::MalformedStructure(format!("{}: Found group `{}` in with position of if. This should have compiled away.", TopDownCompileControl::name(), cond.as_ref().unwrap().borrow().name())));
            }

            // Step 1: Generate the forward edges normally.
            let port_guard = ir::Guard::port(Rc::clone(port));
            let (prevs, nxt) = calculate_states_recur(
                body,
                cur_state,
                prev_states.clone(),
                &port_guard,
                schedule,
                builder,
            )?;

            // Step 2: Generate the backward edges
            // First compute the entry and exit points.
            let mut entries = vec![];
            let mut exits = vec![];
            entries_and_exits(
                body,
                cur_state,
                true,
                true,
                &mut entries,
                &mut exits,
            );
            // For each exit point, generate a map to the guards used.
            let guard_map =
                prevs.clone().into_iter().collect::<HashMap<_, _>>();
            // Generate exit to entry transitions which occur when the condition
            // is true at the end of the while body.
            let exit_to_entry_transitions = exits
                .into_iter()
                .cartesian_product(entries)
                .map(|(old, new)| {
                    (old, new, guard_map[&old].clone() & port_guard.clone())
                });
            schedule.transitions.extend(exit_to_entry_transitions);

            // Step 3: The final out edges from the while come from:
            //   - Before the body when the condition is false
            //   - Inside the body when the condition is false
            let not_port_guard = !port_guard;
            let all_prevs = prev_states
                .into_iter()
                .chain(prevs.into_iter())
                .map(|(st, guard)| (st, guard & not_port_guard.clone()))
                .collect();

            Ok((all_prevs, nxt))
        }
        ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
        ir::Control::Empty(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileEmpty::name()),
        ir::Control::Par(_) => unreachable!(),
    }
}

fn calculate_states(
    con: &ir::Control,
    builder: &mut ir::Builder,
) -> CalyxResult<Schedule> {
    let mut schedule = Schedule::default();
    let (prev, nxt) = calculate_states_recur(
        con,
        0,
        vec![],
        &ir::Guard::True,
        &mut schedule,
        builder,
    )?;
    let transitions = prev.into_iter().map(|(st, guard)| (st, nxt, guard));
    schedule.transitions.extend(transitions);
    Ok(schedule)
}

/// **Core lowering pass.**
/// Compiles away the control programs in components into purely structural
/// code using an finite-state machine (FSM).
///
/// Lowering operates in two steps:
/// 1. Compile all [ir::Par] control sub-programs into a
/// single [ir::Enable] of a group that runs all children
/// to completion.
/// 2. Compile the top-level control program into a single [ir::Enable].
///
/// ## Compiling non-`par` programs
/// Assuming all `par` statements have already been compiled in a control
/// sub-program, we can build a schedule for executing it. We calculate a
/// schedule by assigning an FSM state to each leaf node (an [ir::Enable])
/// as a guard condition. Each control program node also defines a transition
/// function over the states calculated for its children.
///
/// At the end of schedule generation, each FSM state has a set of groups to
/// enable as well as a transition function.
/// This FSM is realized into an implementation using a new group that implements
/// the group enables and the transitions.
///
/// ## Compiling `par` programs
/// We have to generate new FSM-based controller for each child of a `par` node
/// so that each child can indepdendently make progress.
/// If we tie the children to one top-level FSM, their transitions would become
/// interdependent and reduce available concurrency.
///
/// ## Compilation guarantee
/// At the end of this pass, the control program will have no more than one
/// group enable in it.
#[derive(Default)]
pub struct TopDownCompileControl;

impl Named for TopDownCompileControl {
    fn name() -> &'static str {
        "top-down-cc"
    }

    fn description() -> &'static str {
        "Top-down compilation for removing control constructs"
    }
}

impl Visitor for TopDownCompileControl {
    /// Compile each child in `par` block separately so each child can make
    /// progress indepdendently.
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);

        // Compilation group
        let par_group = builder.add_group("par");
        structure!(builder;
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
        );

        // Registers to save the done signal from each child.
        let mut done_regs = Vec::with_capacity(s.stmts.len());

        // For each child, build the enabling logic.
        for con in &s.stmts {
            let group = match con {
                // Do not compile enables
                ir::Control::Enable(ir::Enable { group, .. }) => {
                    Rc::clone(group)
                }
                // Compile complex schedule and return the group.
                _ => {
                    let schedule = calculate_states(con, &mut builder)?;
                    schedule.realize_schedule(&mut builder)
                }
            };

            // Build circuitry to enable and disable this group.
            structure!(builder;
                let pd = prim std_reg(1);
            );
            let group_go = !(guard!(pd["out"]) | guard!(group["done"]));
            let group_done = guard!(group["done"]);

            // Save the done condition in a register.
            let mut assigns = build_assignments!(builder;
                group["go"] = group_go ? signal_on["out"];
                pd["in"] = group_done ? signal_on["out"];
                pd["write_en"] = group_done ? signal_on["out"];
            );
            par_group.borrow_mut().assignments.append(&mut assigns);
            done_regs.push(pd)
        }

        // Done condition for this group
        let done_guard = done_regs
            .clone()
            .into_iter()
            .map(|r| guard!(r["out"]))
            .fold(ir::Guard::True, ir::Guard::and);

        // CLEANUP: Reset the registers once the group is finished.
        let mut cleanup = done_regs
            .into_iter()
            .flat_map(|r| {
                build_assignments!(builder;
                    r["in"] = done_guard ? signal_off["out"];
                    r["write_en"] = done_guard ? signal_on["out"];
                )
            })
            .collect::<Vec<_>>();
        builder
            .component
            .continuous_assignments
            .append(&mut cleanup);

        // Done conditional for this group.
        let done = builder.build_assignment(
            par_group.borrow().get("done"),
            signal_on.borrow().get("out"),
            done_guard,
        );
        par_group.borrow_mut().assignments.push(done);

        Ok(Action::Change(ir::Control::enable(par_group)))
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        // Do not try to compile an enable
        if matches!(
            *comp.control.borrow(),
            ir::Control::Enable(..) | ir::Control::Empty(..)
        ) {
            return Ok(Action::Stop);
        }

        let control = Rc::clone(&comp.control);
        let mut builder = ir::Builder::new(comp, sigs);
        // Add assignments for the final states
        let schedule = calculate_states(&control.borrow(), &mut builder)?;
        schedule.display();
        let comp_group = schedule.realize_schedule(&mut builder);

        Ok(Action::Change(ir::Control::enable(comp_group)))
    }
}
