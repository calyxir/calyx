use super::math_utilities::get_bit_width_from;
use crate::errors::{CalyxResult, Error};
use crate::ir::traversal::ConstructVisitor;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures, Printer, RRC,
};
use crate::{build_assignments, guard, passes, structure};
use itertools::Itertools;
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::ops::Not;
use std::rc::Rc;

/// A range of FSM states.
type Range = (u64, u64);

/// A schedule keeps track of two things:
/// 1. `enables`: Specifies which groups are active during a range of
///     FSM states.
/// 2. `transitions`: Transitions for the FSM registers. A static FSM normally
///    transitions from `state` to `state + 1`. However, special transitions
///    are needed for loops, conditionals, and reseting the FSM.
struct Schedule<'a> {
    enables: HashMap<Range, Vec<ir::Assignment>>,
    transitions: HashSet<(u64, u64, ir::Guard)>,
    builder: &'a mut ir::Builder<'a>,
}

impl<'a> Schedule<'a> {
    fn new(builder: &'a mut ir::Builder<'a>) -> Self {
        Self {
            enables: HashMap::default(),
            transitions: HashSet::default(),
            builder,
        }
    }

    fn last_state(&self) -> u64 {
        assert!(!self.transitions.is_empty(), "Transitions are empty");
        self.transitions.iter().map(|(_, e, _)| *e).max().unwrap()
    }

    fn add_transition(&mut self, start: u64, end: u64, guard: ir::Guard) {
        debug_assert!(
            start != end,
            "Attempting to transition to the same state {start}"
        );
        self.transitions.insert((start, end, guard));
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
                    println!("{start}:")
                } else {
                    println!("[{}, {}):", start, end);
                }
                assigns.iter().for_each(|assign| {
                    print!("  ");
                    Printer::write_assignment(assign, 0, out)
                        .expect("Printing failed!");
                    println!();
                })
            });
        println!("transitions:");
        cond.iter()
            .sorted_by(|(k1, k2, g1), (k3, k4, g2)| {
                k1.cmp(k3).then_with(|| k2.cmp(k4)).then_with(|| g1.cmp(g2))
            })
            .for_each(|(i, f, g)| {
                println!("  ({}, {}): {}", i, f, Printer::guard_str(g));
            });

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
        let lb_const = builder.add_constant(s, fsm_size);
        let ub_const = builder.add_constant(e, fsm_size);
        if s == 0 {
            guard!(fsm["out"]).lt(guard!(ub_const["out"]))
        } else {
            guard!(fsm["out"])
                .ge(guard!(lb_const["out"]))
                .and(guard!(fsm["out"]).lt(guard!(ub_const["out"])))
        }
    }

    fn realize_schedule(self) -> RRC<ir::Group> {
        let final_state = self.last_state();
        let builder = self.builder;
        let (unconditional, conditional) =
            Self::calculate_runs(self.transitions.into_iter());
        let group = builder.add_group("tdst");
        let fsm_size = get_bit_width_from(final_state + 1);

        structure!(builder;
           let fsm = prim std_reg(fsm_size);
           let signal_on = constant(1, 1);
           let first_state = constant(0, fsm_size);
           let last_state = constant(final_state, fsm_size);
        );

        // Enable assignments.
        group.borrow_mut().assignments.extend(
            self.enables
                .into_iter()
                .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
                .flat_map(|((lb, ub), mut assigns)| {
                    let state_guard =
                        Self::range_guard(builder, lb, ub, fsm_size, &fsm);
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

                    let transition_guard = guard!(fsm["out"])
                        .eq(guard!(start_const["out"]))
                        .and(guard);

                    let assigns = build_assignments!(builder;
                        fsm["in"] = transition_guard ? end_const["out"];
                        fsm["write_en"] = transition_guard ? signal_on["out"];
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
                        Self::range_guard(builder, s, e, fsm_size, &fsm);
                    g.or(range)
                },
            );
            structure!(builder;
                let fsm_incr = prim std_add(fsm_size);
                let one = constant(1, fsm_size);
            );
            let uncond_incr = build_assignments!(builder;
                fsm_incr["left"] = ? fsm["out"];
                fsm_incr["right"] = ? one["out"];
                fsm["in"] = uncond_guard ? fsm_incr["out"];
                fsm["write_en"] = uncond_guard ? signal_on["out"];
            );
            group.borrow_mut().assignments.extend(uncond_incr);
        }

        // Done condition for group.
        let last_guard = guard!(fsm["out"]).eq(guard!(last_state["out"]));
        let done_assign = build_assignments!(builder;
            group["done"] = last_guard ? signal_on["out"];
        );
        group.borrow_mut().assignments.extend(done_assign);

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

impl Schedule<'_> {
    fn calculate_states(
        &mut self,
        con: &ir::Control,
        // The current state
        cur_state: u64,
        // Predecessors
        preds: Vec<PredEdge>,
    ) -> CalyxResult<(Vec<PredEdge>, u64)> {
        match con {
        ir::Control::Enable(e) => {
            self.enable_calculate_states(e, cur_state, preds)
        }
        ir::Control::Seq(s) => {
            self.seq_calculate_states(s, cur_state, preds)
        }
        ir::Control::Par(p) => {
            unimplemented!("Static par")
        }
        ir::Control::If(i) => {
            self.if_calculate_states(i, cur_state, preds)
        }
        ir::Control::While(w) => {
            unimplemented!("Static while")
        }
        ir::Control::Invoke(_) => unreachable!(
            "`invoke` statements should have been compiled away. Run `{}` before this pass.",
            passes::CompileInvoke::name()),
        ir::Control::Empty(_) => unreachable!(
            "`empty` statements should have been compiled away. Run `{}` before this pass.",
            passes::CompileEmpty::name()),
    }
    }

    /// Helper to add seqential transitions and return the next state.
    fn seq_add_transitions(
        &mut self,
        preds: &[PredEdge],
        default_pred: &PredEdge,
    ) -> u64 {
        // Compute the new start state from the latest predecessor.
        let new_state = preds
            .iter()
            .max_by_key(|(state, _)| state)
            .unwrap_or(default_pred)
            .0;

        // Add transitions from each predecessor to the new state.
        self.add_transitions(
            preds.iter().map(|(s, g)| (s - 1, new_state, g.clone())),
        );

        // Return the new state.
        new_state
    }

    fn seq_calculate_states(
        &mut self,
        con: &ir::Seq,
        st: u64,
        preds: Vec<PredEdge>,
    ) -> CalyxResult<(Vec<PredEdge>, u64)> {
        let mut cur_preds = preds;
        let mut cur_st = st;
        for stmt in &con.stmts {
            eprintln!("cur_st: {cur_st}, cur_preds: {cur_preds:?}");
            (cur_preds, cur_st) =
                self.calculate_states(stmt, cur_st, cur_preds)?;
        }

        Ok((cur_preds, cur_st))
    }

    /*
    fn par_calculate_states(
        &mut self,
        con: &ir::Par,
        cur_state: u64,
        pre_guard: &ir::Guard,
    ) -> CalyxResult<Vec<PredEdge>> {
        let mut max_state = 0;
        for stmt in &con.stmts {
            let preds = self.calculate_states(stmt, cur_state, pre_guard)?;

            // Compute the start state from the latest predecessor.
            let inner_max_state =
                preds.iter().max_by_key(|(state, _)| state).unwrap().0;

            // Keep track of the latest predecessor state from any statement.
            if inner_max_state > max_state {
                max_state = inner_max_state;
            }
        }

        // Add transitions from the cur_state up to the max_state.
        if cur_state + 1 == max_state {
            self.transitions
                .insert((cur_state, max_state, pre_guard.clone()));
        } else {
            let starts = cur_state..max_state - 1;
            let ends = cur_state + 1..max_state;
            self.transitions.extend(
                starts.zip(ends).map(|(s, e)| (s, e, pre_guard.clone())),
            );
        }

        // Return a single predecessor for the last state.
        Ok(vec![(max_state, pre_guard.clone())])
    }
    */

    /// Compute the states needed for the `if` by allocating a path for the true
    /// branch and another one for the false branch and ensuring it takes the same
    /// amount of time regardless.
    ///
    /// For example:
    /// ```
    /// if lt.out {
    ///     @static(3) tru;
    /// } else {
    ///     @static(5) fal;
    /// }
    /// ```
    ///
    /// Generates transitions:
    /// ```
    /// [0, 3): tru[go] = 1
    /// [3, 5): <empty>
    /// [5, 10): fal[go] = 1
    ///
    /// cond transitions:
    ///   (PREV) -> 0: lt.out
    ///   (PREV) -> 5: !lt.out
    ///
    /// unconditional transitions:
    ///   0 -> 1 -> 2 -> 3 -> 4 -> (EXIT)
    ///   5 -> 6 -> 7 -> 8 -> 9 -> (EXIT)
    /// ```
    ///
    /// Where `PREV` and `EXIT` represent the predecessor and exit states of the
    /// `if` construct.
    fn if_calculate_states(
        &mut self,
        con: &ir::If,
        cur_state: u64,
        preds: Vec<PredEdge>,
    ) -> CalyxResult<(Vec<PredEdge>, u64)> {
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
                        super::RemoveCombGroups::name()))
            .with_pos(&con.attributes));
        }

        let tru_time = *tbranch.get_attribute("static").unwrap();
        let fal_time = *fbranch.get_attribute("static").unwrap();
        let max_time = cmp::max(tru_time, fal_time);

        let port_guard: ir::Guard = Rc::clone(port).into();

        let (mut tpreds, t_nxt) = self.calculate_states(
            tbranch,
            cur_state,
            preds
                .iter()
                .map(|(st, g)| (*st, g.clone() & port_guard.clone()))
                .collect(),
        )?;

        // Balance the true branch if it doesn't have sufficient transitions
        let nxt = if tru_time != max_time {
            // Make all predecessors of the true branch transition to balance state
            self.add_transitions(
                tpreds.into_iter().map(|(st, g)| (st, t_nxt, g)),
            );
            // Add extra transitions from t_nxt to the balance of states
            let balance = max_time - tru_time;
            let last = t_nxt + balance;
            self.add_transitions(
                (t_nxt..last - 1).map(|st| (st, st + 1, ir::Guard::True)),
            );
            tpreds = vec![(last - 1, ir::Guard::True)];
            last
        } else {
            t_nxt
        };

        let f_start = nxt;
        // Compute the false branch transitions by starting from cur_state +
        // max_time since we require the branches to be balanced.
        let (fpreds, nxt_st) = self.calculate_states(
            fbranch,
            f_start,
            preds
                .into_iter()
                .map(|(st, g)| (st, g & !port_guard.clone()))
                .collect(),
        )?;

        if fal_time != max_time {
            unimplemented!("Balancing false branch");
        }

        tpreds.extend(fpreds);

        Ok((tpreds, nxt_st))
    }

    /*
    fn while_calculate_states(
        &mut self,
        con: &ir::While,
        cur_state: u64,
        pre_guard: &ir::Guard,
    ) -> CalyxResult<Vec<PredEdge>> {
        if con.cond.is_some() {
            return Err(Error::pass_assumption(
                    TopDownStaticTiming::name(),
                     format!(
                        "while-with construct should have been compiled away. Run `{}` before this pass.",
                        super::RemoveCombGroups::name()))
            .with_pos(&con.attributes));
        }

        let mut body_exit = cur_state;

        for _ in 0..*con.attributes.get("bound").unwrap() {
            let preds = self.calculate_states(
                &con.body,
                body_exit,
                &pre_guard.clone(),
            )?;

            body_exit = preds
                .iter()
                .max_by_key(|(state, _)| state)
                .unwrap_or(&(body_exit, pre_guard.clone()))
                .0;
        }

        Ok(vec![(body_exit, pre_guard.clone())])
    }
    */

    /// Generate transitions from all predecessors to the enable and keep it
    /// active for its specified static time.
    /// The start state of the enable is computed by taking the max of all
    /// predecessors states.
    fn enable_calculate_states(
        &mut self,
        con: &ir::Enable,
        cur_st: u64,
        preds: Vec<PredEdge>,
    ) -> CalyxResult<(Vec<PredEdge>, u64)> {
        let time_option = con.attributes.get("static");
        let Some(time) = time_option else {
            return Err(Error::pass_assumption(
            TopDownStaticTiming::name(),
            "enable is missing @static annotation. This happens when the enclosing control program has a @static annotation but the enable is missing one.".to_string(),
            ).with_pos(&con.attributes));
        };

        // Transition from all predecessors to the start state
        self.add_transitions(preds.into_iter().map(|(st, g)| (st, cur_st, g)));

        // Activate the group during the latency
        let last_st = cur_st + time;
        let range = (cur_st, last_st);
        let group = &con.group;
        structure!(self.builder;
            let signal_on = constant(1, 1);
        );
        let mut assigns = build_assignments!(self.builder;
            group["go"] = ? signal_on["out"];
        );
        self.enables.entry(range).or_default().append(&mut assigns);

        // Add any necessary internal transitions. In the case time is 1 and there
        // is a single transition, it is taken care of by the parent.
        self.add_transitions(
            (cur_st..last_st - 1).map(|s| (s, s + 1, ir::Guard::True)),
        );

        Ok((vec![(last_st - 1, ir::Guard::True)], last_st))
    }
}

/// Lowering pass that generates latency-sensitive FSMs when control sub-programs have `@static`
/// annotations. The pass works opportunisitically and attempts to compile all nested static
/// control programs nested within the overall program, replacing them with groups that implement
/// the correct transitions.
///
/// `while` control blocks can only be statically compiled when they additionally have a `@bound`
/// annotation which mentions the expected number of times a loop will iterate.
pub struct TopDownStaticTiming {
    /// Print out the FSM representation to STDOUT.
    dump_fsm: bool,
}

impl ConstructVisitor for TopDownStaticTiming {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(&["dump-fsm"], ctx);

        Ok(TopDownStaticTiming { dump_fsm: opts[0] })
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

impl Visitor for TopDownStaticTiming {
    fn start_seq(
        &mut self,
        con: &mut ir::Seq,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let time_option = con.attributes.get("static");

        // If sub-tree is not static, skip this node.
        if time_option.is_none() {
            return Ok(Action::Continue);
        }

        // Compile control program and save schedule.
        let mut builder = ir::Builder::new(comp, sigs);
        let mut schedule = Schedule::new(&mut builder);
        schedule.seq_calculate_states(con, 0, vec![])?;

        // Dump FSM if requested.
        if self.dump_fsm {
            schedule.display();
        }

        // Realize the schedule in a replacement control group.
        let group = schedule.realize_schedule();

        Ok(Action::change(ir::Control::enable(group)))
    }

    fn start_par(
        &mut self,
        con: &mut ir::Par,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let time_option = con.attributes.get("static");

        // If sub-tree is not static, skip this node.
        if time_option.is_none() {
            return Ok(Action::Continue);
        }

        unimplemented!("Compilation of par static");

        /*
        // Compile control program and save schedule.
        let mut builder = ir::Builder::new(comp, sigs);
        let mut schedule = Schedule::new(&mut builder);
        schedule.par_calculate_states(con, 0, &ir::Guard::True)?;

        // Dump FSM if requested.
        if self.dump_fsm {
            schedule.display();
        }

        // Realize the schedule in a replacement control group.
        let group = schedule.realize_schedule();

        Ok(Action::change(ir::Control::enable(group)))
        */
    }

    fn start_while(
        &mut self,
        con: &mut ir::While,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let time_option = con.attributes.get("static");
        let bound_option = con.attributes.get("option");

        // If sub-tree is not static, skip this node.
        if time_option.is_none() || bound_option.is_none() {
            return Ok(Action::Continue);
        }
        unimplemented!("Compilation of while static")

        /*
        // Compile control program and save schedule.
        let mut builder = ir::Builder::new(comp, sigs);
        let mut schedule = Schedule::new(&mut builder);
        schedule.while_calculate_states(con, 0, &ir::Guard::True)?;

        // Dump FSM if requested.
        if self.dump_fsm {
            schedule.display();
        }

        // Realize the schedule in a replacement control group.
        let group = schedule.realize_schedule();

        Ok(Action::change(ir::Control::enable(group)))
        */
    }

    fn start_if(
        &mut self,
        con: &mut ir::If,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let time_option = con.attributes.get("static");

        // If sub-tree is not static, skip this node.
        if time_option.is_none() {
            return Ok(Action::Continue);
        }

        // Compile control program and save schedule.
        let mut builder = ir::Builder::new(comp, sigs);
        let mut schedule = Schedule::new(&mut builder);
        schedule.if_calculate_states(con, 0, vec![])?;

        // Dump FSM if requested.
        if self.dump_fsm {
            schedule.display();
        }

        // Realize the schedule in a replacement control group.
        let group = schedule.realize_schedule();

        Ok(Action::change(ir::Control::enable(group)))
    }
}
