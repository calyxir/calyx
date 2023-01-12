use super::compute_states::{END, START};
use crate::analysis::WithStatic;
use crate::errors::{CalyxResult, Error};
use crate::ir::traversal::ConstructVisitor;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures, Printer, RRC,
};
use crate::ir::{CloneName, GetAttributes};
use crate::passes::top_down_static_timing::{ComputeStates, Normalize};
use crate::passes::{self, math_utilities::get_bit_width_from};
use crate::{build_assignments, guard, structure};
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::iter;
use std::ops::Not;
use std::rc::Rc;

use super::compute_states::ID;

/// A range of FSM states.
type Range = (u64, u64);

/// A schedule keeps track of two things:
/// 1. `enables`: Specifies which groups are active during a range of
///     FSM states.
/// 2. `transitions`: Transitions for the FSM registers. A static FSM normally
///    transitions from `state` to `state + 1`. However, special transitions
///    are needed for loops, conditionals, and reseting the FSM.
struct Schedule<'b, 'a: 'b> {
    /// Enable assignments in a particular range
    enables: HashMap<Range, Vec<ir::Assignment>>,
    /// Transition from one state to another when a guard is true
    transitions: HashSet<(u64, u64, ir::Guard)>,
    // Builder for the associated component
    builder: &'b mut ir::Builder<'a>,
    states: ComputeStates,
}

impl<'b, 'a: 'b> Schedule<'b, 'a> {
    fn new(builder: &'b mut ir::Builder<'a>, states: ComputeStates) -> Self {
        Self {
            enables: HashMap::default(),
            transitions: HashSet::new(),
            builder,
            states,
        }
    }
}

impl Schedule<'_, '_> {
    fn last(&self) -> u64 {
        debug_assert!(!self.transitions.is_empty());
        self.transitions.iter().max_by_key(|(_, e, _)| e).unwrap().1
    }

    /// Add a new transition between the range [start, end).
    fn add_transition(&mut self, start: u64, end: u64, guard: ir::Guard) {
        debug_assert!(
            !(start == end && guard.is_true()),
            "Unconditional transition to the same state {start}"
        );
        self.transitions.insert((start, end, guard));
    }

    // Add enables that are active in the range [start, end).
    // Automatically ignores any enable statements that refer to the balance group.
    fn add_enables(
        &mut self,
        start: u64,
        end: u64,
        assigns: impl IntoIterator<Item = ir::Assignment>,
    ) {
        debug_assert!(
            start != end,
            "Attempting to enable groups in empty range [{start}, {start})"
        );
        self.enables
            .entry((start, end))
            .or_default()
            .extend(assigns);
    }

    fn add_transitions(
        &mut self,
        transitions: impl Iterator<Item = (u64, u64, ir::Guard)>,
    ) {
        transitions.for_each(|(s, e, g)| self.add_transition(s, e, g));
    }

    fn display(&self) {
        let out = &mut std::io::stdout();
        let (uncond, cond) =
            Self::calculate_runs(self.transitions.iter().cloned());

        self.enables
            .iter()
            .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .for_each(|((start, end), assigns)| {
                if *end == start + 1 {
                    println!("{}:", start);
                } else {
                    println!("[{}, {}):", start, end);
                }
                assigns.iter().for_each(|assign| {
                    print!("  ");
                    Printer::write_assignment(assign, 0, out)
                        .expect("Printing failed!");
                    println!();
                });
                if assigns.is_empty() {
                    println!("  <empty>");
                }
            });
        if !cond.is_empty() {
            println!("transitions:");
            cond.iter()
                .sorted_by(|(k1, k2, g1), (k3, k4, g2)| {
                    k1.cmp(k3).then_with(|| k2.cmp(k4)).then_with(|| g1.cmp(g2))
                })
                .for_each(|(i, f, g)| {
                    println!("  ({}, {}): {}", i, f, Printer::guard_str(g));
                });
        }

        // Unconditional +1 transitions
        if !uncond.is_empty() {
            let uncond_trans = uncond
                .into_iter()
                .map(|(s, e)| format!("({}, {})", s, e))
                .join(", ");
            println!("Unconditional runs:\n  {}", uncond_trans);
        }
    }

    /// Returns "runs" of FSM states where transitions happen unconditionally
    fn calculate_runs<I>(
        transitions: I,
    ) -> (Vec<Range>, Vec<(u64, u64, ir::Guard)>)
    where
        I: Iterator<Item = (u64, u64, ir::Guard)>,
    {
        // XXX(rachit): This only works for "true" guards and fails to compress if there is any
        // other guard. For example, if there is a sequence under a conditional branch, this will
        // fail to compress that sequence.
        let (unconditional, conditional): (Vec<_>, Vec<_>) = transitions
            .partition(|(s, e, guard)| *e == s + 1 && guard.is_true());

        let mut unconditional =
            unconditional.into_iter().map(|(s, _, _)| s).sorted();

        let mut ranges: Vec<Range> = Vec::new();
        if let Some(mut cur_s) = unconditional.next() {
            let mut start_s = cur_s;

            // Extract the next state
            for nxt_s in unconditional {
                if nxt_s != cur_s + 1 {
                    ranges.push((start_s, cur_s + 1));
                    start_s = nxt_s;
                }
                cur_s = nxt_s
            }
            ranges.push((start_s, cur_s + 1));
        }

        (ranges, conditional)
    }

    fn range_guard(
        builder: &mut ir::Builder,
        s: u64,
        e: u64,
        fsm_size: u64,
        fsm: &RRC<ir::Cell>,
    ) -> ir::Guard {
        structure!(builder;
            let lb_const = constant(s, fsm_size);
            let ub_const = constant(e, fsm_size);
        );
        if s == 0 {
            guard!(fsm["out"]).lt(guard!(ub_const["out"]))
        } else if e == s + 1 {
            guard!(fsm["out"]).eq(guard!(lb_const["out"]))
        } else if e == 1 << fsm_size {
            guard!(fsm["out"]).ge(guard!(lb_const["out"]))
        } else {
            guard!(fsm["out"])
                .ge(guard!(lb_const["out"]))
                .and(guard!(fsm["out"]).lt(guard!(ub_const["out"])))
        }
    }

    /// Construct hardware to implement the given schedule.
    ///
    /// Requires the outgoing edges from the control program and the final state of the FSM.
    /// All the hardware is instantiated using the builder associated with this schedule.
    fn realize_schedule(
        self,
        mut out_edges: Vec<PredEdge>,
        dump_fsm: bool,
    ) -> RRC<ir::Group> {
        let last = self.last();
        if dump_fsm {
            self.display();
        }

        let builder = self.builder;
        let (unconditional, conditional) =
            Self::calculate_runs(self.transitions.into_iter());
        let group = builder.add_group("tdst");
        let fsm_size = get_bit_width_from(last + 1);

        structure!(builder;
           let st_fsm = prim std_reg(fsm_size);
           let signal_on = constant(1, 1);
           let first_state = constant(0, fsm_size);
        );

        // Enable assignments.
        group.borrow_mut().assignments.extend(
            self.enables
                .into_iter()
                .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
                .flat_map(|((lb, ub), mut assigns)| {
                    let state_guard =
                        Self::range_guard(builder, lb, ub, fsm_size, &st_fsm);
                    assigns.iter_mut().for_each(|assign| {
                        assign.guard.update(|g| g.and(state_guard.clone()))
                    });
                    assigns
                }),
        );

        // Conditional Transition assignments.
        group.borrow_mut().assignments.extend(
            conditional
                .into_iter()
                .sorted_by_key(|(start, _, _)| *start)
                .flat_map(|(start, end, guard)| {
                    structure!(builder;
                        let start_const = constant(start, fsm_size);
                        let end_const = constant(end, fsm_size);
                    );

                    let transition_guard = guard!(st_fsm["out"])
                        .eq(guard!(start_const["out"]))
                        .and(guard);

                    let assigns = build_assignments!(builder;
                        st_fsm["in"] = transition_guard ? end_const["out"];
                        st_fsm["write_en"] = transition_guard ? signal_on["out"];
                    );
                    assigns
                }),
        );
        // Unconditional Transitions
        if !unconditional.is_empty() {
            let uncond_guard: ir::Guard = unconditional.into_iter().fold(
                ir::Guard::True.not(),
                |g, (s, e)| {
                    let range =
                        Self::range_guard(builder, s, e, fsm_size, &st_fsm);
                    g.or(range)
                },
            );
            structure!(builder;
                let fsm_incr = prim std_add(fsm_size);
                let one = constant(1, fsm_size);
            );
            let uncond_incr = build_assignments!(builder;
                fsm_incr["left"] = ? st_fsm["out"];
                fsm_incr["right"] = ? one["out"];
                st_fsm["in"] = uncond_guard ? fsm_incr["out"];
                st_fsm["write_en"] = uncond_guard ? signal_on["out"];
            );
            group.borrow_mut().assignments.extend(uncond_incr);
        }

        // Done condition for group.
        let (st, g) = out_edges.pop().expect("No outgoing edges");
        let c = builder.add_constant(st, fsm_size);
        let mut done_guard = guard!(st_fsm["out"]).eq(guard!(c["out"])) & g;
        for (st, g) in out_edges {
            let stc = builder.add_constant(st, fsm_size);
            let st_guard = guard!(st_fsm["out"]).eq(guard!(stc["out"]));
            done_guard |= st_guard & g;
        }
        let done_assign = build_assignments!(builder;
            group["done"] = done_guard ? signal_on["out"];
        );
        group.borrow_mut().assignments.extend(done_assign);

        // Cleanup: Add a transition from last state to the first state.
        let reset_fsm = build_assignments!(builder;
            st_fsm["in"] = done_guard ? first_state["out"];
            st_fsm["write_en"] = done_guard ? signal_on["out"];
        );
        // Reset all loop indices to 0
        let reset_indices = self
            .states
            .indices()
            .flat_map(|c: RRC<ir::Cell>| {
                let size = c.borrow().get_parameter("WIDTH").unwrap();
                let zero = builder.add_constant(0, size);
                let assigns = build_assignments!(builder;
                    c["in"] = done_guard ? zero["out"];
                    c["write_en"] = done_guard ? signal_on["out"];
                );
                assigns
            })
            .collect_vec();
        builder
            .component
            .continuous_assignments
            .extend(reset_fsm.into_iter().chain(reset_indices));

        group
    }
}

/// Represents an edge from a predeccesor to the current control node.
/// The `u64` represents the FSM state of the predeccesor and the guard needs
/// to be true for the predeccesor to transition to the current state.
type PredEdge = (u64, ir::Guard);

impl Schedule<'_, '_> {
    fn calculate_states(
        &mut self,
        con: &mut ir::Control,
        // Predecessors
        preds: Vec<PredEdge>,
    ) -> CalyxResult<Vec<PredEdge>> {
        debug_assert!(!preds.is_empty(), "Predecessors should not be empty.");

        match con {
            ir::Control::Enable(e) => {
                self.enable_calculate_states(e, preds)
            }
            ir::Control::Seq(s) => {
                self.seq_calculate_states(s, preds)
            }
            ir::Control::If(i) => {
                self.if_calculate_states(i, preds)
            }
            ir::Control::While(w) => {
                self.while_calculate_states(w, preds)
            }
            ir::Control::Par(par) => {
                self.par_calculate_states(par, preds)
            }
            ir::Control::Invoke(_) => unreachable!(
                "`invoke` statements should have been compiled away. Run `{}` before this pass.",
                passes::CompileInvoke::name()),
            ir::Control::Empty(_) => unreachable!(
                "`empty` statements should have been compiled away. Run `{}` before this pass.",
                passes::CompileEmpty::name()),
        }
    }

    /// Generate transitions from all predecessors to the enable and keep it
    /// active for its specified static time.
    /// The start state of the enable is computed by taking the max of all
    /// predecessors states.
    fn enable_calculate_states(
        &mut self,
        con: &ir::Enable,
        preds: Vec<PredEdge>,
    ) -> CalyxResult<Vec<PredEdge>> {
        let time_option = con.attributes.get("static");
        let Some(&time) = time_option else {
            return Err(Error::pass_assumption(
        TopDownStaticTiming::name(),
            "enable is missing @static annotation. This happens when the enclosing control program has a @static annotation but the enable is missing one.".to_string(),
            ).with_pos(&con.attributes));
        };

        let cur_st = con.attributes[ID];

        // Enable the group during the transition. Note that this is similar to
        // what tdcc does the early transitions flag. However, unlike tdcc, we
        // know that transitions do not use groups' done signals.
        preds.clone().into_iter().for_each(|(st, g)| {
            let group = &con.group;
            structure!(self.builder;
                let signal_on = constant(1, 1);
            );
            let assigns = build_assignments!(self.builder;
                group["go"] = g ? signal_on["out"];
            );
            // We only enable this in the state when the transition starts
            self.add_enables(st, st + 1, assigns);
        });

        // Transition from all predecessors to the start state
        self.add_transitions(preds.into_iter().map(|(st, g)| (st, cur_st, g)));

        // Activate the group during the latency. Subtract one because the group
        // is also active during the transition when not in the start state.
        let last_st = cur_st + time - 1;
        // Add additional transitions if the group's latency is not 1
        if time != 1 {
            let group = &con.group;
            structure!(self.builder;
                let signal_on = constant(1, 1);
            );
            let assigns = build_assignments!(self.builder;
                group["go"] = ? signal_on["out"];
            );
            self.add_enables(cur_st, last_st, assigns);

            // Add any necessary internal transitions. In the case time is 1 and there
            // is a single transition, it is taken care of by the parent.
            self.add_transitions(
                (cur_st..last_st).map(|s| (s, s + 1, ir::Guard::True)),
            );
        }

        Ok(vec![(last_st, ir::Guard::True)])
    }

    fn seq_calculate_states(
        &mut self,
        con: &mut ir::Seq,
        preds: Vec<PredEdge>,
    ) -> CalyxResult<Vec<PredEdge>> {
        let mut cur_preds = preds;
        for stmt in &mut con.stmts {
            cur_preds = self.calculate_states(stmt, cur_preds)?;
        }

        Ok(cur_preds)
    }

    /// Requires that all the threads are group enables.
    /// Compilation simply compiles each enable with the current predecessors as
    /// they must all start executing at the same time.
    ///
    /// They will all add transitions to their end time, possibly duplicating
    /// transition edges but the group with the longest latency will add all the
    /// needed transitions for last state.
    fn par_calculate_states(
        &mut self,
        con: &mut ir::Par,
        preds: Vec<PredEdge>,
    ) -> CalyxResult<Vec<PredEdge>> {
        for stmt in &mut con.stmts {
            if let ir::Control::Enable(en) = stmt {
                self.enable_calculate_states(en, preds.clone())?;
            } else {
                unreachable!("Par should only contain enables")
            }
        }
        Ok(vec![(
            con.attributes[ID] + con.attributes["static"] - 1,
            ir::Guard::True,
        )])
    }

    /// Compute the states needed for the `if` by allocating a path for the true
    /// branch and another one for the false branch and ensuring it takes the same
    /// amount of time regardless.
    ///
    /// For example:
    /// ```
    /// <PREV>
    /// if lt.out {
    ///     @static(3) tru;
    /// } else {
    ///     @static(5) fal;
    /// }
    /// <EXIT>
    /// ```
    ///
    /// We need to ensure that the previous group has finished performing its
    /// computation before transitions to either the true or false branch.
    ///
    /// TODO: Add documentation
    ///
    /// Where `PREV` and `EXIT` represent the predecessor and exit states of the
    /// `if` construct.
    fn if_calculate_states(
        &mut self,
        con: &mut ir::If,
        preds: Vec<PredEdge>,
    ) -> CalyxResult<Vec<PredEdge>> {
        let ir::If {
            port,
            cond,
            tbranch,
            fbranch,
            ..
        } = con;
        if cond.is_some() {
            return Err(Error::pass_assumption(
                    TopDownStaticTiming::name(),
                     format!(
                        "if-with construct should have been compiled away. Run `{}` before this pass.",
                        passes::RemoveCombGroups::name()))
            .with_pos(&con.attributes));
        }

        let port_guard: ir::Guard = Rc::clone(port).into();
        let mut tpreds = self.calculate_states(
            tbranch,
            preds
                .iter()
                .map(|(st, g)| (*st, g.clone() & port_guard.clone()))
                .collect(),
        )?;

        // Compute the false branch transitions by starting from the end of the true branch states
        let fpreds = self.calculate_states(
            fbranch,
            preds
                .into_iter()
                .map(|(st, g)| (st, g & !port_guard.clone()))
                .collect(),
        )?;

        tpreds.extend(fpreds);

        Ok(tpreds)
    }

    /// Define a group that increments a counter every cycle
    fn incr_group(&mut self, idx: &RRC<ir::Cell>) -> RRC<ir::Group> {
        let group =
            self.builder.add_group(format!("incr_{}", idx.clone_name()));
        let size = idx.borrow().get_parameter("WIDTH").unwrap();
        structure!(self.builder;
            let st_incr = prim std_add(size);
            let one = constant(1, size);
            let on = constant(1, 1);
        );
        let incr_assigns = build_assignments!(self.builder;
            st_incr["left"] = ? idx["out"];
            st_incr["right"] = ? one["out"];
            idx["in"] = ? st_incr["out"];
            idx["write_en"] =  ? on["out"];
            group["done"] = ? idx["done"];
        );
        group.borrow_mut().assignments = incr_assigns;
        group
    }

    /// Define a group that resets a counter to 0
    fn reset_group(&mut self, idx: &RRC<ir::Cell>) -> RRC<ir::Group> {
        let group = self
            .builder
            .add_group(format!("reset_{}", idx.clone_name()));
        let size = idx.borrow().get_parameter("WIDTH").unwrap();
        structure!(self.builder;
            let zero = constant(0, size);
            let on = constant(1, 1);
        );
        let assigns = build_assignments!(self.builder;
            idx["in"] = ? zero["out"];
            idx["write_en"] = ? on["out"];
            group["done"] = ? idx["done"];
        );
        group.borrow_mut().assignments = assigns;
        group
    }

    /// Compute the transitions for a bounded while loop.
    /// Iterations are guaranteed to execute the cycle right after the body
    /// finishes executing.
    ///
    /// Instantiates a counter that increments every cycle while the `while` loop is active and exits the
    /// loop body when the counter reaches `body*bound`.
    ///
    /// For example:
    /// ```
    /// @bound(10) while lt.out {
    ///   @static(1) one;
    ///   @static(2) two;
    /// }
    /// ```
    ///
    /// Generates the following transitions:
    /// ```
    /// [0, 1):
    ///     one[go] = 1
    ///     idx = idx < 10 : idx + 1 : 0
    /// [1, 3): two[go] = 1
    ///
    /// cond transitions:
    ///   (PREV) -> 0: idx < 10
    ///   2 -> 0:      idx < 10
    ///   2 -> (EXIT): idx == 10
    /// ```
    fn while_calculate_states(
        &mut self,
        wh: &mut ir::While,
        preds: Vec<PredEdge>,
    ) -> CalyxResult<Vec<PredEdge>> {
        if wh.cond.is_some() {
            return Err(Error::pass_assumption(
            TopDownStaticTiming::name(),
            format!(
                "while-with construct should have been compiled away. Run `{}` before this pass.",
                passes::RemoveCombGroups::name())
            ).with_pos(&wh.attributes));
        }

        // Construct the index and incrementing logic.
        let bound = wh.attributes["bound"];
        // Loop bound should not be less than 1.
        if bound < 1 {
            return Err(Error::malformed_structure(
                "Loop bound is less than 1",
            )
            .with_pos(&wh.attributes));
        }

        let (idx, total) = self.states.loop_bounds(wh, self.builder);
        structure!(self.builder;
            let on = constant(1, 1);
        );
        // Add back edges
        let enter_guard = guard!(idx["out"]).lt(guard!(total["out"]));
        let mut exits = vec![];
        self.states
            .control_exits(&wh.body, self.builder, &mut exits);
        let back_edges = exits
            .clone()
            .into_iter()
            .map(|(st, g)| (st, g & enter_guard.clone()));

        // Compute the body transitions.
        let body_preds = self.calculate_states(
            &mut wh.body,
            preds.clone().into_iter().chain(back_edges).collect_vec(),
        )?;

        // Index incrementing logic
        let incr_group = self.incr_group(&idx);
        let incr_activate = self.builder.build_assignment(
            incr_group.borrow().get("go"),
            on.borrow().get("out"),
            enter_guard,
        );
        // Even though the assignments are active during [cur_state, body_nxt), we expect only `bound*body` number of
        // states will actually be traversed internally.
        let cur_state = wh.attributes[START];
        let body_nxt = wh.attributes[END];
        self.add_enables(
            cur_state,
            body_nxt,
            iter::once(incr_activate.clone()),
        );
        // Activate increment assignments while transitioning into the loop
        for (st, guard) in preds {
            let mut assign = incr_activate.clone();
            *assign.guard &= guard.clone();
            self.add_enables(st, st + 1, iter::once(assign));
        }

        // Reset the index when exiting the loop
        let exit = guard!(idx["out"]).eq(guard!(total["out"]));
        let reset_group = self.reset_group(&idx);
        let reset_activate = self.builder.build_assignment(
            reset_group.borrow().get("go"),
            on.borrow().get("out"),
            exit.clone(),
        );
        for (st, _) in exits {
            // Ensure that reset assignments are active when exiting the loop.
            self.add_enables(st, st + 1, iter::once(reset_activate.clone()));
        }

        let exits = body_preds
            .into_iter()
            .map(|(st, g)| (st, g & exit.clone()))
            .collect_vec();

        Ok(exits)
    }
}

impl Schedule<'_, '_> {
    fn compile(
        con: &mut ir::Control,
        builder: &mut ir::Builder,
        dump_fsm: bool,
    ) -> CalyxResult<RRC<ir::Group>> {
        debug_assert!(
            con.get_attribute("static").is_some(),
            "Attempted to compile non-static program"
        );
        // Normalize the program
        Normalize::apply(con, builder);
        // Compute the states associated with the program
        let states = ComputeStates::new(con, builder);
        // Generate a schedule for this program
        let mut schedule = Schedule::new(builder, states);
        let out_edges =
            schedule.calculate_states(con, vec![(0, ir::Guard::True)])?;
        Ok(schedule.realize_schedule(out_edges, dump_fsm))
    }
}

/// **Core Lowering Pass**: Generates latency-sensitive FSMs when control sub-programs have `@static`.
/// Must be invoked for programs that need to use cycle-level reasoning. Expects that combinational
/// groups and invoke statements have been compiled away.
///
/// Compilation proceeds in the following high-level steps:
/// 1. *Normalization*: Ensures all `if` branches are balanced, i.e. take the same number of cycles,
///    and directly nested, bounded while loops are de-nested.
/// 2. *State computation*: Assigns states to enables based on their timing.
/// 3. *FSM Generation*: Generates FSM for each static control program and replaces the sub-program
///    with an enable for the group implementing the schedule.
///
/// The pass provides strong guarantees on cycle-level execution of groups unlike [passes::TopDownCompileControl].
/// - `seq { a; b; c }`: `b` starts execution exactly in the `a` is done.
/// - `if port { t } else { f }`: Either branch will start executing as soon as the `if` program starts executing.
/// - `@bound(n) while port { b }`: Each iteration starts execution exactly when the previous iteration is done.
/// - `par { a; b }`: `a` and `b` start executing in the same cycle.
///
/// ## Compilation
/// Like [passes::TopDownCompileControl], this pass first walks over the control
/// program and compiles all `@static par` control programs by allocating each
/// thread in the `par` with a separate FSM.
///
/// After this first traversal, the pass walks over the control program again
/// and compiles each sub-program is marked as `@static`.
/// [Schedule] encapsulates the compilation logic for each supported compilation operator.
pub struct TopDownStaticTiming {
    /// Print out the FSM representation to STDOUT.
    dump_fsm: bool,
    /// Make sure that the program is fully compiled by this pass
    force: bool,
}

impl ConstructVisitor for TopDownStaticTiming {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(&["dump-fsm", "force"], ctx);

        Ok(TopDownStaticTiming {
            dump_fsm: opts[0],
            force: opts[1],
        })
    }

    fn clear_data(&mut self) {
        /* All data can be transferred between components */
    }
}

impl Named for TopDownStaticTiming {
    fn name() -> &'static str {
        "tdst"
    }

    fn description() -> &'static str {
        "Top-down latency-sensitive compilation for removing control constructs"
    }
}

impl TopDownStaticTiming {
    fn compile_sub_programs(
        con: &mut ir::Control,
        builder: &mut ir::Builder,
        dump_fsm: bool,
    ) -> CalyxResult<()> {
        // Do not attempt to compile Enable and Empty statement
        if matches!(con, ir::Control::Enable(_) | ir::Control::Empty(_)) {
            return Ok(());
        }
        if let Some(time) = con.get_attribute("static") {
            let group = Schedule::compile(con, builder, dump_fsm)?;
            let mut en = ir::Control::enable(group);
            en.get_mut_attributes()["static"] = time;
            *con = en;
        } else {
            match con {
                ir::Control::Seq(ir::Seq { stmts, .. })
                | ir::Control::Par(ir::Par { stmts, .. }) => {
                    for stmt in stmts.iter_mut() {
                        Self::compile_sub_programs(stmt, builder, dump_fsm)?;
                    }
                }
                ir::Control::If(ir::If {
                    tbranch, fbranch, ..
                }) => {
                    Self::compile_sub_programs(tbranch, builder, dump_fsm)?;
                    Self::compile_sub_programs(fbranch, builder, dump_fsm)?;
                }
                ir::Control::While(ir::While { body, .. }) => {
                    Self::compile_sub_programs(body, builder, dump_fsm)?;
                }
                ir::Control::Enable(_)
                | ir::Control::Invoke(_)
                | ir::Control::Empty(_) => {}
            }
        }
        Ok(())
    }
}

impl Visitor for TopDownStaticTiming {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut con = comp.control.borrow_mut();
        // Dont compile empty or single-enable control programs
        if matches!(&*con, ir::Control::Enable(_) | ir::Control::Empty(_)) {
            return Ok(Action::Stop);
        }
        // Propagate all static information through the control program.
        con.update_static(&HashMap::new());
        Ok(Action::Continue)
    }
    fn finish_par(
        &mut self,
        con: &mut ir::Par,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if !con.attributes.has("static") {
            return Ok(Action::Continue);
        }
        let mut builder = ir::Builder::new(comp, sigs);
        // Ensure that all threads in the `par` block are group enables
        for stmt in &mut con.stmts {
            match stmt {
                ir::Control::Enable(_) => {}
                con => {
                    let time = con.get_attribute("static").unwrap();
                    let group =
                        Schedule::compile(con, &mut builder, self.dump_fsm)?;
                    let mut en = ir::Control::enable(group);
                    en.get_mut_attributes()["static"] = time;
                    *con = en;
                }
            }
        }
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Take ownership of the control program
        let mut con = comp.control.replace(ir::Control::empty());

        // Compile all sub-programs
        let mut builder = ir::Builder::new(comp, sigs);
        Self::compile_sub_programs(&mut con, &mut builder, self.dump_fsm)?;

        // Add the control program back.
        comp.control = Rc::new(RefCell::new(con));

        // If the force flag is set, make sure that we only have one group remaining
        if self.force
            && !matches!(&*comp.control.borrow(), ir::Control::Enable(_))
        {
            return Err(Error::pass_assumption(
                Self::name(),
                "`force` flag was set but the final control program is not an enable"
            ));
        }
        Ok(Action::Continue)
    }
}
