use super::math_utilities::get_bit_width_from;
use crate::errors::CalyxResult;
use crate::ir::traversal::ConstructVisitor;
use crate::ir::GetAttributes;
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
use std::io::Write;
use std::rc::Rc;

const NODE_ID: &str = "NODE_ID";

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
/// The exit point is `cond0`.
///
/// Multiple exit points are created when conditions are used:
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
/// The exit set is `[true, false]`.
fn control_exits(
    con: &ir::Control,
    cur_state: u64,
    is_exit: bool,
    exits: &mut Vec<(u64, RRC<ir::Group>)>,
) -> u64 {
    match con {
        ir::Control::Enable(ir::Enable { group, .. }) => {
            if is_exit {
                exits.push((cur_state, Rc::clone(group)))
            }
            cur_state + 1
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            let len = stmts.len();
            let mut cur = cur_state;
            for (idx, stmt) in stmts.iter().enumerate() {
                let exit = idx == len - 1 && is_exit;
                cur = control_exits(stmt, cur, exit, exits);
            }
            cur
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            let tru_nxt = control_exits(
                tbranch, cur_state, is_exit, exits,
            );
            control_exits(
                fbranch, tru_nxt, is_exit, exits,
            )
        }
        ir::Control::While(ir::While { body, .. }) => control_exits(
            body, cur_state, is_exit, exits,
        ),
        ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
        ir::Control::Empty(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileEmpty::name()),
        ir::Control::Par(_) => unreachable!(),
    }
}

fn compute_unique_ids(con: &mut ir::Control, cur_state: u64) -> u64 {
    match con {
        ir::Control::Enable(ir::Enable { attributes, .. }) => {
            attributes.insert(NODE_ID, cur_state);
            cur_state + 1
        }
        ir::Control::Par(ir::Par { stmts, attributes }) => {
            attributes.insert(NODE_ID, cur_state);
            stmts.iter_mut().for_each(|stmt| {
                compute_unique_ids(stmt, 0);
            });
            cur_state + 1
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            let mut cur = cur_state;
            stmts.iter_mut().for_each(|stmt| {
                cur = compute_unique_ids(stmt, cur);
            });
            cur
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            // If the program starts with a branch then branches can't get
            // the initial state.
            let cur_state = if cur_state == 0 {
                cur_state + 1
            } else {
                cur_state
            };
            let tru_nxt = compute_unique_ids(
                tbranch, cur_state
            );
            compute_unique_ids(
                fbranch, tru_nxt
            )
        }
        ir::Control::While(ir::While { body, .. }) => {
            // If the program starts with a branch then branches can't get
            // the initial state.
            let cur_state = if cur_state == 0 {
                cur_state + 1
            } else {
                cur_state
            };
            compute_unique_ids(body, cur_state)
        }
        ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
        ir::Control::Empty(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileEmpty::name()),
    }
}

/// Represents the dyanmic execution schedule of a control program.
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
    fn display(&self, group: String) {
        let out = &mut std::io::stdout();
        writeln!(out, "======== {} =========", group).unwrap();
        self.enables
            .iter()
            .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .for_each(|(state, assigns)| {
                writeln!(out, "{}:", state).unwrap();
                assigns.iter().for_each(|assign| {
                    IRPrinter::write_assignment(assign, 2, out).unwrap();
                    writeln!(out).unwrap();
                })
            });
        writeln!(out, "{}:\n  <end>", self.last_state()).unwrap();
        writeln!(out, "transitions:").unwrap();
        self.transitions
            .iter()
            .sorted_by(|(k1, _, _), (k2, _, _)| k1.cmp(k2))
            .for_each(|(i, f, g)| {
                writeln!(out, "  ({}, {}): {}", i, f, IRPrinter::guard_str(g))
                    .unwrap();
            });
    }

    /// Implement a given [Schedule] and return the name of the [ir::Group] that
    /// implements it.
    fn realize_schedule(
        self,
        group: RRC<ir::Group>,
        builder: &mut ir::Builder,
    ) -> RRC<ir::Group> {
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

/// Represents an edge from a predeccesor to the current control node.
/// The `u64` represents the FSM state of the predeccesor and the guard needs
/// to be true for the predeccesor to transition to the current state.
type PredEdge = (u64, ir::Guard);

/// Recursively build an dynamic finite state machine represented by a [Schedule].
/// Does the following, given an [ir::Control]:
///     1. If needed, add transitions from predeccesors to the current state.
///     2. Enable the groups in the current state
///     3. Calculate [PredEdge] implied by this state
///     4. Return [PredEdge] and the next state.
fn calculate_states_recur(
    con: &ir::Control,
    // The current state
    cur_state: u64,
    // The set of previous states that want to transition into cur_state
    preds: Vec<(u64, ir::Guard)>,
    // Current schedule.
    schedule: &mut Schedule,
    // Component builder
    builder: &mut ir::Builder,
    // True if early_transitions are allowed
    early_transitions: bool,
) -> CalyxResult<(Vec<PredEdge>, u64)> {
    match con {
        // See explanation of FSM states generated in [ir::TopDownCompileControl].
        ir::Control::Enable(ir::Enable { group, attributes }) => {
            let cur_state = *attributes.get(NODE_ID).unwrap_or_else(|| panic!("Group `{}` does not have node_id information", group.borrow().name()));
            // If there is exactly one previous transition state with a `true`
            // guard, then merge this state into previous state.
            // This happens when the first control statement is an enable not
            // inside a branch.
            let (cur_state, prev_states) = if preds.len() == 1 && preds[0].1.is_true() {
                (preds[0].0, vec![])
            } else {
                (cur_state, preds)
            };

            let not_done = !guard!(group["done"]);
            let signal_on = builder.add_constant(1, 1);

            // Activate this group in the current state
            let mut en_go = build_assignments!(builder;
                group["go"] = not_done ? signal_on["out"];
            );
            schedule
                .enables
                .entry(cur_state)
                .or_default()
                .append(&mut en_go);

            // Activate group in the cycle when previous state signals done.
            // NOTE: We explicilty do not add `not_done` to the guard.
            // See explanation in [ir::TopDownCompileControl] to understand
            // why.
            if early_transitions {
            for (st, g) in &prev_states {
                let mut early_go = build_assignments!(builder;
                    group["go"] = g ? signal_on["out"];
                );
                schedule.enables.entry(*st).or_default().append(&mut early_go);
            }
            }

            let transitions = prev_states
                .into_iter()
                .map(|(st, guard)| (st, cur_state, guard));
            schedule.transitions.extend(transitions);

            let done_cond = guard!(group["done"]);
            let nxt = cur_state + 1;
            Ok((vec![(cur_state, done_cond)], nxt))
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            let mut prev = preds;
            let mut cur = cur_state;
            for stmt in stmts {
                let res = calculate_states_recur(
                    stmt,
                    cur,
                    prev,
                    schedule,
                    builder,
                    early_transitions
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
            let port_guard: ir::Guard = Rc::clone(port).into();
            // Previous states transitioning into true branch need the conditional
            // to be true.
            let tru_transitions = preds.clone().into_iter().map(|(s, g)| (s, g & port_guard.clone())).collect();
            let (tru_prev, tru_nxt) = calculate_states_recur(
                tbranch,
                cur_state,
                tru_transitions,
                schedule,
                builder,
                early_transitions
            )?;
            // Previous states transitioning into false branch need the conditional
            // to be false.
            let fal_transitions = preds.into_iter().map(|(s, g)| (s, g & !port_guard.clone())).collect();
            let (fal_prev, fal_nxt) = calculate_states_recur(
                fbranch,
                tru_nxt,
                fal_transitions,
                schedule,
                builder,
                early_transitions
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

            let port_guard: ir::Guard = Rc::clone(port).into();

            // Step 1: Generate the backward edges
            // First compute the entry and exit points.
            let mut exits = vec![];
            control_exits(
                body,
                cur_state,
                true,
                &mut exits,
            );
            let back_edge_prevs = exits.into_iter().map(|(st, group)| (st, group.borrow().get("done").into()));

            // Step 2: Generate the forward edges normally.
            // Previous transitions into the body require the condition to be
            // true.
            let transitions: Vec<(u64, ir::Guard)> = preds
                .clone()
                .into_iter()
                .chain(back_edge_prevs)
                .map(|(s, g)| (s, g & port_guard.clone()))
                .collect();
            let (prevs, nxt) = calculate_states_recur(
                body,
                cur_state,
                transitions,
                schedule,
                builder,
                early_transitions
            )?;

            // Step 3: The final out edges from the while come from:
            //   - Before the body when the condition is false
            //   - Inside the body when the condition is false
            let not_port_guard = !port_guard;
            let all_prevs = preds
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
    early_transitions: bool,
) -> CalyxResult<Schedule> {
    let mut schedule = Schedule::default();
    let first_state = (0, ir::Guard::True);
    // We create an empty first state in case the control program starts with
    // a branch (if, while).
    // If the program doesn't branch, then the initial state is merged into
    // the first group.
    let (prev, nxt) = calculate_states_recur(
        con,
        1,
        vec![first_state],
        &mut schedule,
        builder,
        early_transitions,
    )?;
    let transitions = prev.into_iter().map(|(st, guard)| (st, nxt, guard));
    schedule.transitions.extend(transitions);
    Ok(schedule)
}

/// **Core lowering pass.**
/// Compiles away the control programs in components into purely structural code using an
/// finite-state machine (FSM).
///
/// Lowering operates in two steps:
/// 1. Compile all [ir::Par] control sub-programs into a single [ir::Enable] of a group that runs
///    all children to completion.
/// 2. Compile the top-level control program into a single [ir::Enable].
///
/// ## Compiling non-`par` programs
/// At very high-level, the pass assigns an FSM state to each [ir::Enable] in the program and
/// generates transitions to the state to activate the groups contained within the [ir::Enable].
///
/// The compilation process calculates all predeccesors of the [ir::Enable] while walking over the
/// control program. A predeccesor is any enable statement that can directly "jump" to the current
/// [ir::Enable]. The compilation process computes all such predeccesors and the guards that need
/// to be true for the predeccesor to jump into this enable statement.
///
/// ```
/// cond0;
/// while lt.out {
///   if gt.out { true } else { false }
/// }
/// next;
/// ```
/// The predeccesor sets are:
/// ```
/// cond0 -> []
/// true -> [(cond0, lt.out & gt.out); (true; lt.out & gt.out); (false, lt.out & !gt.out)]
/// false -> [(cond0, lt.out & !gt.out); (true; lt.out & gt.out); (false, lt.out & !gt.out)]
/// next -> [(cond0, !lt.out); (true, !lt.out); (false, !lt.out)]
/// ```
///
/// ### Compiling [ir::Enable]
/// The process first takes all edges from predeccesors and transitions to the state for this
/// enable and enables the group in this state:
/// ```text
/// let cur_state; // state of this enable
/// for (state, guard) in predeccesors:
///   transitions.insert(state, cur_state, guard)
/// enables.insert(cur_state, group)
/// ```
///
/// While this process will generate a functioning FSM, the FSM takes unnecessary cycles for FSM
/// transitions.
///
/// For example:
/// ```
/// seq { one; two; }
/// ```
/// The FSM generated will look like this (where `f` is the FSM register):
/// ```
/// f.in = one[done] ? 1;
/// f.in = two[done] ? 2;
/// one[go] = !one[done] & f.out == 0;
/// two[go] = !two[done] & f.out == 1;
/// ```
///
/// The cycle-level timing for this FSM will look like:
///     - cycle 0: (f.out == 0), enable one
///     - cycle t: (f.out == 0), (one[done] == 1), disable one
///     - cycle t+1: (f.out == 1), enable two
///     - cycle t+l: (f.out == 1), (two[done] == 1), disable two
///     - cycle t+l+1: finish
///
/// The transition t -> t+1 represents one where group one is done but group two hasn't started
/// executing.
///
/// To address this specific problem, there is an additional enable added to run all groups within
/// an enable *while the FSM is transitioning*.
/// The final transition will look like this:
/// ```
/// f.in = one[done] ? 1;
/// f.in = two[done] ? 2;
/// one[go] = !one[done] & f.out == 0;
/// two[go] = (!two[done] & f.out == 1) || (one[done] & f.out == 0);
/// ```
///
/// Note that `!two[done]` isn't present in the second disjunct because all groups are guaranteed
/// to run for at least one cycle and the second disjunct will only be true for one cycle before
/// the first disjunct becomes true.
///
/// ## Compiling `par` programs
/// We have to generate new FSM-based controller for each child of a `par` node so that each child
/// can indepdendently make progress.
/// If we tie the children to one top-level FSM, their transitions would become interdependent and
/// reduce available concurrency.
///
/// ## Compilation guarantee
/// At the end of this pass, the control program will have no more than one
/// group enable in it.
pub struct TopDownCompileControl {
    /// Print out the FSM representation to STDOUT
    dump_fsm: bool,
    /// Disable early transitions
    no_early_transitions: bool,
}

impl ConstructVisitor for TopDownCompileControl {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let mut dump_fsm = false;
        let mut no_early_transitions = false;
        ctx.extra_opts.iter().for_each(|opt| {
            let mut splits = opt.split(':');
            if splits.next() == Some(Self::name()) {
                match splits.next() {
                    Some("dump-fsm") => {
                        dump_fsm = true;
                    }
                    Some("no-early-transitions") => {
                        no_early_transitions = true;
                    }
                    _ => (),
                }
            }
        });
        Ok(TopDownCompileControl {
            dump_fsm,
            no_early_transitions,
        })
    }

    fn clear_data(&mut self) {
        /* All data can be transferred between components */
    }
}

impl Named for TopDownCompileControl {
    fn name() -> &'static str {
        "tdcc"
    }

    fn description() -> &'static str {
        "Top-down compilation for removing control constructs"
    }
}

impl Visitor for TopDownCompileControl {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        let mut con = comp.control.borrow_mut();
        compute_unique_ids(&mut con, 0);
        // IRPrinter::write_control(&con, 0, &mut std::io::stderr());
        Ok(Action::Continue)
    }

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
                    let schedule = calculate_states(
                        con,
                        &mut builder,
                        !self.no_early_transitions,
                    )?;
                    let group = builder.add_group("tdcc");
                    if self.dump_fsm {
                        schedule.display(format!(
                            "{}:{}",
                            builder.component.name,
                            group.borrow().name()
                        ));
                    }
                    schedule.realize_schedule(group, &mut builder)
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

        // Add NODE_ID to compiled group.
        let mut en = ir::Control::enable(par_group);
        let node_id = s.attributes.get(NODE_ID).unwrap();
        en.get_mut_attributes().unwrap().insert(NODE_ID, *node_id);

        Ok(Action::Change(en))
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
        // IRPrinter::write_control(&control.borrow(), 0, &mut std::io::stderr());
        let mut builder = ir::Builder::new(comp, sigs);
        // Add assignments for the final states
        let schedule = calculate_states(
            &control.borrow(),
            &mut builder,
            !self.no_early_transitions,
        )?;
        let group = builder.add_group("tdcc");
        if self.dump_fsm {
            schedule.display(format!(
                "{}:{}",
                builder.component.name,
                group.borrow().name()
            ));
        }
        let comp_group = schedule.realize_schedule(group, &mut builder);

        Ok(Action::Change(ir::Control::enable(comp_group)))
    }
}
