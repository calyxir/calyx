use super::math_utilities::get_bit_width_from;
use crate::passes;
use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir, GetAttributes, LibrarySignatures, Printer, RRC};
use calyx_ir::{build_assignments, guard, structure};
use calyx_utils::CalyxResult;
use calyx_utils::Error;
use ir::Nothing;
use itertools::Itertools;
use petgraph::graph::DiGraph;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

const NODE_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NODE_ID);

/// Computes the exit edges of a given [ir::Control] program.
///
/// ## Example
/// In the following Calyx program:
/// ```
/// while comb_reg.out {
///   seq {
///     @NODE_ID(4) incr;
///     @NODE_ID(5) cond0;
///   }
/// }
/// ```
/// The exit edge is is `[(5, cond0[done])]` indicating that the state 5 exits when the guard
/// `cond0[done]` is true.
///
/// Multiple exit points are created when conditions are used:
/// ```
/// while comb_reg.out {
///   @NODE_ID(7) incr;
///   if comb_reg2.out {
///     @NODE_ID(8) tru;
///   } else {
///     @NODE_ID(9) fal;
///   }
/// }
/// ```
/// The exit set is `[(8, tru[done] & !comb_reg.out), (9, fal & !comb_reg.out)]`.
fn control_exits(con: &ir::Control, exits: &mut Vec<PredEdge>) {
    match con {
        ir::Control::Empty(_) => {}
        ir::Control::Enable(ir::Enable { group, attributes }) => {
            let cur_state = attributes.get(NODE_ID).unwrap();
            exits.push((cur_state, guard!(group["done"])))
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            if let Some(stmt) = stmts.last() { control_exits(stmt, exits) }
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            control_exits(
                tbranch, exits,
            );
            control_exits(
                fbranch, exits,
            )
        }
        ir::Control::While(ir::While { body, port, .. }) => {
            let mut loop_exits = vec![];
            control_exits(body, &mut loop_exits);
            // Loop exits only happen when the loop guard is false
            exits.extend(loop_exits.into_iter().map(|(s, g)| {
                (s, g & !ir::Guard::from(port.clone()))
            }));
        },
        ir::Control::Repeat(_) => unreachable!("`repeat` statements should have been compiled away. Run `{}` before this pass.", passes::CompileRepeat::name()),
        ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
        ir::Control::Par(_) => unreachable!(),
        ir::Control::Static(_) => unreachable!(" static control should have been compiled away. Run the static compilation passes before this pass")
    }
}

/// Adds the @NODE_ID attribute to [ir::Enable] and [ir::Par].
/// Each [ir::Enable] gets a unique label within the context of a child of
/// a [ir::Par] node.
/// Furthermore, if an if/while/seq statement is labeled with a `new_fsm` attribute,
/// then it will get its own unique label. Within that if/while/seq, each enable
/// will get its own unique label within the context of that if/while/seq (see
/// example for clarification).
///
/// ## Example:
/// ```
/// seq { A; B; par { C; D; }; E; @new_fsm seq {F; G; H}}
/// ```
/// gets the labels:
/// ```
/// seq {
///   @NODE_ID(1) A; @NODE_ID(2) B;
///   @NODE_ID(3) par {
///     @NODE_ID(0) C;
///     @NODE_ID(0) D;
///   }
///   @NODE_ID(4) E;
///   @NODE_ID(5) seq{
///     @NODE_ID(0) F;
///     @NODE_ID(1) G;
///     @NODE_ID(2) H;
///   }
/// }
/// ```
///
/// These identifiers are used by the compilation methods [calculate_states_recur]
/// and [control_exits].
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
        ir::Control::Seq(ir::Seq { stmts, attributes }) => {
            let new_fsm = attributes.has(ir::BoolAttr::NewFSM);
            // if new_fsm is true, then insert attribute at the seq, and then
            // start over counting states from 0
            let mut cur = if new_fsm{
                attributes.insert(NODE_ID, cur_state);
                0
            } else {
                cur_state
            };
            stmts.iter_mut().for_each(|stmt| {
                cur = compute_unique_ids(stmt, cur);
            });
            // If new_fsm is true then we want to return cur_state + 1, since this
            // seq should really only take up 1 "state" on the "outer" fsm
            if new_fsm{
                cur_state + 1
            } else {
                cur
            }
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, attributes, ..
        }) => {
            let new_fsm = attributes.has(ir::BoolAttr::NewFSM);
            // if new_fsm is true, then we want to add an attribute to this
            // control statement
            if new_fsm {
                attributes.insert(NODE_ID, cur_state);
            }
            // If the program starts with a branch then branches can't get
            // the initial state.
            // Also, if new_fsm is true, we want to start with state 1 as well:
            // we can't start at 0 for the reason mentioned above
            let cur = if new_fsm || cur_state == 0 {
                1
            } else {
                cur_state
            };
            let tru_nxt = compute_unique_ids(
                tbranch, cur
            );
            let false_nxt = compute_unique_ids(
                fbranch, tru_nxt
            );
            // If new_fsm is true then we want to return cur_state + 1, since this
            // if stmt should really only take up 1 "state" on the "outer" fsm
            if new_fsm {
                cur_state + 1
            } else {
                false_nxt
            }
        }
        ir::Control::While(ir::While { body, attributes, .. }) => {
            let new_fsm = attributes.has(ir::BoolAttr::NewFSM);
            // if new_fsm is true, then we want to add an attribute to this
            // control statement
            if new_fsm{
                attributes.insert(NODE_ID, cur_state);
            }
            // If the program starts with a branch then branches can't get
            // the initial state.
            // Also, if new_fsm is true, we want to start with state 1 as well:
            // we can't start at 0 for the reason mentioned above
            let cur = if new_fsm || cur_state == 0 {
                1
            } else {
                cur_state
            };
            let body_nxt = compute_unique_ids(body, cur);
            // If new_fsm is true then we want to return cur_state + 1, since this
            // while loop should really only take up 1 "state" on the "outer" fsm
            if new_fsm{
                cur_state + 1
            } else {
                body_nxt
            }
        }
        ir::Control::Empty(_) => cur_state,
        ir::Control::Repeat(_) => unreachable!("`repeat` statements should have been compiled away. Run `{}` before this pass.", passes::CompileRepeat::name()),
        ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
        ir::Control::Static(_) => unreachable!("static control should have been compiled away. Run the static compilation passes before this pass")
    }
}

/// Represents the dyanmic execution schedule of a control program.
struct Schedule<'b, 'a: 'b> {
    /// Assigments that should be enabled in a given state.
    pub enables: HashMap<u64, Vec<ir::Assignment<Nothing>>>,
    /// Transition from one state to another when the guard is true.
    pub transitions: Vec<(u64, u64, ir::Guard<Nothing>)>,
    /// The component builder. The reference has a shorter lifetime than the builder itself
    /// to allow multiple schedules to use the same builder.
    pub builder: &'b mut ir::Builder<'a>,
}

impl<'b, 'a> From<&'b mut ir::Builder<'a>> for Schedule<'b, 'a> {
    fn from(builder: &'b mut ir::Builder<'a>) -> Self {
        Schedule {
            enables: HashMap::new(),
            transitions: Vec::new(),
            builder,
        }
    }
}

impl<'b, 'a> Schedule<'b, 'a> {
    /// Validate that all states are reachable in the transition graph.
    fn validate(&self) {
        let graph = DiGraph::<(), u32>::from_edges(
            self.transitions
                .iter()
                .map(|(s, e, _)| (*s as u32, *e as u32)),
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
                    Printer::write_assignment(assign, 2, out).unwrap();
                    writeln!(out).unwrap();
                })
            });
        writeln!(out, "{}:\n  <end>", self.last_state()).unwrap();
        writeln!(out, "transitions:").unwrap();
        self.transitions
            .iter()
            .sorted_by(|(k1, _, _), (k2, _, _)| k1.cmp(k2))
            .for_each(|(i, f, g)| {
                writeln!(out, "  ({}, {}): {}", i, f, Printer::guard_str(g))
                    .unwrap();
            });
    }

    /// Implement a given [Schedule] and return the name of the [ir::Group] that
    /// implements it.
    fn realize_schedule(self, dump_fsm: bool) -> RRC<ir::Group> {
        self.validate();

        let group = self.builder.add_group("tdcc");
        if dump_fsm {
            self.display(format!(
                "{}:{}",
                self.builder.component.name,
                group.borrow().name()
            ));
        }

        let final_state = self.last_state();
        let fsm_size = get_bit_width_from(
            final_state + 1, /* represent 0..final_state */
        );
        structure!(self.builder;
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
                    let state_const =
                        self.builder.add_constant(state, fsm_size);
                    let state_guard = guard!(fsm["out"] == state_const["out"]);
                    assigns.iter_mut().for_each(|asgn| {
                        asgn.guard.update(|g| g.and(state_guard.clone()))
                    });
                    assigns
                }),
        );

        // Transition assignments
        group.borrow_mut().assignments.extend(
            self.transitions.into_iter().flat_map(|(s, e, guard)| {
                structure!(self.builder;
                    let end_const = constant(e, fsm_size);
                    let start_const = constant(s, fsm_size);
                );
                let ec_borrow = end_const.borrow();
                let trans_guard =
                    guard!((fsm["out"] == start_const["out"]) & guard);

                vec![
                    self.builder.build_assignment(
                        fsm.borrow().get("in"),
                        ec_borrow.get("out"),
                        trans_guard.clone(),
                    ),
                    self.builder.build_assignment(
                        fsm.borrow().get("write_en"),
                        signal_on.borrow().get("out"),
                        trans_guard,
                    ),
                ]
            }),
        );

        // Done condition for group
        let last_guard = guard!(fsm["out"] == last_state["out"]);
        let done_assign = self.builder.build_assignment(
            group.borrow().get("done"),
            signal_on.borrow().get("out"),
            last_guard.clone(),
        );
        group.borrow_mut().assignments.push(done_assign);

        // Cleanup: Add a transition from last state to the first state.
        let reset_fsm = build_assignments!(self.builder;
            fsm["in"] = last_guard ? first_state["out"];
            fsm["write_en"] = last_guard ? signal_on["out"];
        );
        self.builder
            .component
            .continuous_assignments
            .extend(reset_fsm);

        group
    }
}

/// Represents an edge from a predeccesor to the current control node.
/// The `u64` represents the FSM state of the predeccesor and the guard needs
/// to be true for the predeccesor to transition to the current state.
type PredEdge = (u64, ir::Guard<Nothing>);

impl Schedule<'_, '_> {
    /// Recursively build an dynamic finite state machine represented by a [Schedule].
    /// Does the following, given an [ir::Control]:
    ///     1. If needed, add transitions from predeccesors to the current state.
    ///     2. Enable the groups in the current state
    ///     3. Calculate [PredEdge] implied by this state
    ///     4. Return [PredEdge] and the next state.
    /// Another note: the functions calc_seq_recur, calc_while_recur, and calc_if_recur
    /// are functions that `calculate_states_recur` uses for when con is a seq, while,
    /// and if respectively. The reason why they are defined as separate functions is because we
    /// need to call `calculate_seq_recur` (for example) directly when we are in `finish_seq`
    /// since `finish_seq` only gives us access to a `& mut seq` type, not a `& Control`
    /// type.
    fn calculate_states_recur(
        // Current schedule.
        &mut self,
        con: &ir::Control,
        // The set of previous states that want to transition into cur_state
        preds: Vec<PredEdge>,
        // True if early_transitions are allowed
        early_transitions: bool,
    ) -> CalyxResult<Vec<PredEdge>> {
        match con {
        // See explanation of FSM states generated in [ir::TopDownCompileControl].
        ir::Control::Enable(ir::Enable { group, attributes }) => {
            let cur_state = attributes.get(NODE_ID).unwrap_or_else(|| panic!("Group `{}` does not have node_id information", group.borrow().name()));
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
            let signal_on = self.builder.add_constant(1, 1);

            // Activate this group in the current state
            let en_go = build_assignments!(self.builder;
                group["go"] = not_done ? signal_on["out"];
            );
            self
                .enables
                .entry(cur_state)
                .or_default()
                .extend(en_go);

            // Activate group in the cycle when previous state signals done.
            // NOTE: We explicilty do not add `not_done` to the guard.
            // See explanation in [ir::TopDownCompileControl] to understand
            // why.
            if early_transitions {
                for (st, g) in &prev_states {
                    let early_go = build_assignments!(self.builder;
                        group["go"] = g ? signal_on["out"];
                    );
                    self.enables.entry(*st).or_default().extend(early_go);
                }
            }

            let transitions = prev_states
                .into_iter()
                .map(|(st, guard)| (st, cur_state, guard));
            self.transitions.extend(transitions);

            let done_cond = guard!(group["done"]);
            Ok(vec![(cur_state, done_cond)])
        }
        ir::Control::Seq(seq) => {
            self.calc_seq_recur(seq, preds, early_transitions)
        }
        ir::Control::If(if_stmt) => {
            self.calc_if_recur(if_stmt, preds, early_transitions)
        }
        ir::Control::While(while_stmt) => {
            self.calc_while_recur(while_stmt, preds, early_transitions)
        }
        ir::Control::Par(_) => unreachable!(),
        ir::Control::Repeat(_) => unreachable!("`repeat` statements should have been compiled away. Run `{}` before this pass.", passes::CompileRepeat::name()),
        ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
        ir::Control::Empty(_) => unreachable!("`calculate_states_recur` should not see an `empty` control."),
        ir::Control::Static(_) => unreachable!("static control should have been compiled away. Run the static compilation passes before this pass")
    }
    }

    /// Builds a finite state machine for `seq` represented by a [Schedule].
    /// At a high level, it iterates through each stmt in the seq's control, using the
    /// previous stmt's [PredEdge] as the `preds` for the current stmt, and returns
    /// the [PredEdge] implied by the last stmt in `seq`'s control.
    fn calc_seq_recur(
        &mut self,
        seq: &ir::Seq,
        // The set of previous states that want to transition into cur_state
        preds: Vec<PredEdge>,
        // True if early_transitions are allowed
        early_transitions: bool,
    ) -> CalyxResult<Vec<PredEdge>> {
        let mut prev = preds;
        for stmt in &seq.stmts {
            prev =
                self.calculate_states_recur(stmt, prev, early_transitions)?;
        }
        Ok(prev)
    }

    /// Builds a finite state machine for `if_stmt` represented by a [Schedule].
    /// First generates the transitions into the true branch + the transitions that exist
    /// inside the true branch. Then generates the transitions into the false branch + the transitions
    /// that exist inside the false branch. Then calculates the transitions needed to
    /// exit the if statmement (which include edges from both the true and false branches).
    fn calc_if_recur(
        &mut self,
        if_stmt: &ir::If,
        // The set of previous states that want to transition into cur_state
        preds: Vec<PredEdge>,
        // True if early_transitions are allowed
        early_transitions: bool,
    ) -> CalyxResult<Vec<PredEdge>> {
        if if_stmt.cond.is_some() {
            return Err(Error::malformed_structure(format!("{}: Found group `{}` in with position of if. This should have compiled away.", TopDownCompileControl::name(), if_stmt.cond.as_ref().unwrap().borrow().name())));
        }
        let port_guard: ir::Guard<Nothing> = Rc::clone(&if_stmt.port).into();
        // Previous states transitioning into true branch need the conditional
        // to be true.
        let tru_transitions = preds
            .clone()
            .into_iter()
            .map(|(s, g)| (s, g & port_guard.clone()))
            .collect();
        let tru_prev = self.calculate_states_recur(
            &if_stmt.tbranch,
            tru_transitions,
            early_transitions,
        )?;
        // Previous states transitioning into false branch need the conditional
        // to be false.
        let fal_transitions = preds
            .into_iter()
            .map(|(s, g)| (s, g & !port_guard.clone()))
            .collect();

        let fal_prev = if let ir::Control::Empty(..) = *if_stmt.fbranch {
            // If the false branch is empty, then all the prevs to this node will become prevs
            // to the next node.
            fal_transitions
        } else {
            self.calculate_states_recur(
                &if_stmt.fbranch,
                fal_transitions,
                early_transitions,
            )?
        };

        let prevs = tru_prev.into_iter().chain(fal_prev).collect();
        Ok(prevs)
    }

    /// Builds a finite state machine for `while_stmt` represented by a [Schedule].
    /// It first generates the backwards edges (i.e., edges from the end of the while
    /// body back to the beginning of the while body), then generates the forwards
    /// edges in the body, then generates the edges that exit the while loop.
    fn calc_while_recur(
        &mut self,
        while_stmt: &ir::While,
        // The set of previous states that want to transition into cur_state
        preds: Vec<PredEdge>,
        // True if early_transitions are allowed
        early_transitions: bool,
    ) -> CalyxResult<Vec<PredEdge>> {
        if while_stmt.cond.is_some() {
            return Err(Error::malformed_structure(format!("{}: Found group `{}` in with position of if. This should have compiled away.", TopDownCompileControl::name(), while_stmt.cond.as_ref().unwrap().borrow().name())));
        }

        let port_guard: ir::Guard<Nothing> = Rc::clone(&while_stmt.port).into();

        // Step 1: Generate the backward edges by computing the exit nodes.
        let mut exits = vec![];
        control_exits(&while_stmt.body, &mut exits);

        // Step 2: Generate the forward edges normally.
        // Previous transitions into the body require the condition to be
        // true.
        let transitions: Vec<PredEdge> = preds
            .clone()
            .into_iter()
            .chain(exits)
            .map(|(s, g)| (s, g & port_guard.clone()))
            .collect();
        let prevs = self.calculate_states_recur(
            &while_stmt.body,
            transitions,
            early_transitions,
        )?;

        // Step 3: The final out edges from the while come from:
        //   - Before the body when the condition is false
        //   - Inside the body when the condition is false
        let not_port_guard = !port_guard;
        let all_prevs = preds
            .into_iter()
            .chain(prevs)
            .map(|(st, guard)| (st, guard & not_port_guard.clone()))
            .collect();

        Ok(all_prevs)
    }

    /// Creates a Schedule that represents `seq`, mainly relying on `calc_seq_recur()`.
    fn calculate_states_seq(
        &mut self,
        seq: &ir::Seq,
        early_transitions: bool,
    ) -> CalyxResult<()> {
        let first_state = (0, ir::Guard::True);
        // We create an empty first state in case the control program starts with
        // a branch (if, while).
        // If the program doesn't branch, then the initial state is merged into
        // the first group.
        let prev =
            self.calc_seq_recur(seq, vec![first_state], early_transitions)?;
        self.add_nxt_transition(prev);
        Ok(())
    }

    /// Creates a Schedule that represents `if`, mainly relying on `calc_if_recur()`.
    fn calculate_states_if(
        &mut self,
        if_stmt: &ir::If,
        early_transitions: bool,
    ) -> CalyxResult<()> {
        let first_state = (0, ir::Guard::True);
        // We create an empty first state in case the control program starts with
        // a branch (if, while).
        // If the program doesn't branch, then the initial state is merged into
        // the first group.
        let prev =
            self.calc_if_recur(if_stmt, vec![first_state], early_transitions)?;
        self.add_nxt_transition(prev);
        Ok(())
    }

    /// Creates a Schedule that represents `while`, mainly relying on `calc_while_recur()`.
    fn calculate_states_while(
        &mut self,
        while_stmt: &ir::While,
        early_transitions: bool,
    ) -> CalyxResult<()> {
        let first_state = (0, ir::Guard::True);
        // We create an empty first state in case the control program starts with
        // a branch (if, while).
        // If the program doesn't branch, then the initial state is merged into
        // the first group.
        let prev = self.calc_while_recur(
            while_stmt,
            vec![first_state],
            early_transitions,
        )?;
        self.add_nxt_transition(prev);
        Ok(())
    }

    /// Given predecessors prev, creates a new "next" state and transitions from
    /// each state in prev to the next state.
    /// In other words, it just adds an "end" state to [Schedule] and the
    /// appropriate transitions to that "end" state.
    fn add_nxt_transition(&mut self, prev: Vec<PredEdge>) {
        let nxt = prev
            .iter()
            .max_by(|(st1, _), (st2, _)| st1.cmp(st2))
            .unwrap()
            .0
            + 1;
        let transitions = prev.into_iter().map(|(st, guard)| (st, nxt, guard));
        self.transitions.extend(transitions);
    }

    /// Note: the functions calculate_states_seq, calculate_states_while, and calculate_states_if
    /// are functions that basically do what `calculate_states` would do if `calculate_states` knew (for certain)
    /// that its input parameter would be a seq/while/if.
    /// The reason why we need to define these as separate functions is because `finish_seq`
    /// (for example) we only gives us access to a `& mut seq` type, not a `& Control`
    /// type.
    fn calculate_states(
        &mut self,
        con: &ir::Control,
        early_transitions: bool,
    ) -> CalyxResult<()> {
        let first_state = (0, ir::Guard::True);
        // We create an empty first state in case the control program starts with
        // a branch (if, while).
        // If the program doesn't branch, then the initial state is merged into
        // the first group.
        let prev = self.calculate_states_recur(
            con,
            vec![first_state],
            early_transitions,
        )?;
        self.add_nxt_transition(prev);
        Ok(())
    }
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
///     - cycle 0: (`f.out` == 0), enable one
///     - cycle t: (`f.out` == 0), (`one[done]` == 1), disable one
///     - cycle t+1: (`f.out` == 1), enable two
///     - cycle t+l: (`f.out` == 1), (`two[done]` == 1), disable two
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
    /// Enable early transitions
    early_transitions: bool,
}

impl ConstructVisitor for TopDownCompileControl {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);

        Ok(TopDownCompileControl {
            dump_fsm: opts[&"dump-fsm"].bool(),
            early_transitions: opts[&"early-transitions"].bool(),
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

    fn opts() -> Vec<PassOpt> {
        vec![
            PassOpt::new(
                "dump-fsm",
                "Print out the state machine implementing the schedule",
                ParseVal::Bool(false),
                PassOpt::parse_bool,
            ),
            PassOpt::new(
                "early-transitions",
                "Experimental: Enable early transitions for group enables",
                ParseVal::Bool(false),
                PassOpt::parse_bool,
            ),
        ]
    }
}

impl Visitor for TopDownCompileControl {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Do not try to compile an enable
        if matches!(
            *comp.control.borrow(),
            ir::Control::Enable(..) | ir::Control::Empty(..)
        ) {
            return Ok(Action::Stop);
        }

        let mut con = comp.control.borrow_mut();
        compute_unique_ids(&mut con, 0);
        // IRPrinter::write_control(&con, 0, &mut std::io::stderr());
        Ok(Action::Continue)
    }

    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // only compile using new fsm if has new_fsm attribute
        if !s.attributes.has(ir::BoolAttr::NewFSM) {
            return Ok(Action::Continue);
        }
        let mut builder = ir::Builder::new(comp, sigs);
        let mut sch = Schedule::from(&mut builder);
        sch.calculate_states_seq(s, self.early_transitions)?;
        // Compile schedule and return the group.
        let seq_group = sch.realize_schedule(self.dump_fsm);

        // Add NODE_ID to compiled group.
        let mut en = ir::Control::enable(seq_group);
        let node_id = s.attributes.get(NODE_ID).unwrap();
        en.get_mut_attributes().insert(NODE_ID, node_id);

        Ok(Action::change(en))
    }

    fn finish_if(
        &mut self,
        i: &mut ir::If,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // only compile using new fsm if has new_fsm attribute
        if !i.attributes.has(ir::BoolAttr::NewFSM) {
            return Ok(Action::Continue);
        }
        let mut builder = ir::Builder::new(comp, sigs);
        let mut sch = Schedule::from(&mut builder);

        // Compile schedule and return the group.
        sch.calculate_states_if(i, self.early_transitions)?;
        let if_group = sch.realize_schedule(self.dump_fsm);

        // Add NODE_ID to compiled group.
        let mut en = ir::Control::enable(if_group);
        let node_id = i.attributes.get(NODE_ID).unwrap();
        en.get_mut_attributes().insert(NODE_ID, node_id);

        Ok(Action::change(en))
    }

    fn finish_while(
        &mut self,
        w: &mut ir::While,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // only compile using new fsm if has attribute
        if !w.attributes.has(ir::BoolAttr::NewFSM) {
            return Ok(Action::Continue);
        }
        let mut builder = ir::Builder::new(comp, sigs);
        let mut sch = Schedule::from(&mut builder);
        sch.calculate_states_while(w, self.early_transitions)?;

        // Compile schedule and return the group.
        let if_group = sch.realize_schedule(self.dump_fsm);

        // Add NODE_ID to compiled group.
        let mut en = ir::Control::enable(if_group);
        let node_id = w.attributes.get(NODE_ID).unwrap();
        en.get_mut_attributes().insert(NODE_ID, node_id);

        Ok(Action::change(en))
    }

    /// Compile each child in `par` block separately so each child can make
    /// progress indepdendently.
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
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
                    let mut sch = Schedule::from(&mut builder);
                    sch.calculate_states(con, self.early_transitions)?;
                    sch.realize_schedule(self.dump_fsm)
                }
            };

            // Build circuitry to enable and disable this group.
            structure!(builder;
                let pd = prim std_reg(1);
            );
            let group_go = !(guard!(pd["out"] | group["done"]));
            let group_done = guard!(group["done"]);

            // Save the done condition in a register.
            let assigns = build_assignments!(builder;
                group["go"] = group_go ? signal_on["out"];
                pd["in"] = group_done ? signal_on["out"];
                pd["write_en"] = group_done ? signal_on["out"];
            );
            par_group.borrow_mut().assignments.extend(assigns);
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
        en.get_mut_attributes().insert(NODE_ID, node_id);

        Ok(Action::change(en))
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let control = Rc::clone(&comp.control);
        // IRPrinter::write_control(&control.borrow(), 0, &mut std::io::stderr());
        let mut builder = ir::Builder::new(comp, sigs);
        let mut sch = Schedule::from(&mut builder);
        // Add assignments for the final states
        sch.calculate_states(&control.borrow(), self.early_transitions)?;
        let comp_group = sch.realize_schedule(self.dump_fsm);

        Ok(Action::change(ir::Control::enable(comp_group)))
    }
}
