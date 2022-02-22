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
        self.transitions.iter().map(|(_, e, _)| *e).max().unwrap()
    }

    fn display(&self) {
        let out = &mut std::io::stdout();
        println!("enables:");
        self.enables
            .iter()
            .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .for_each(|((start, end), assigns)| {
                println!("[{}, {})", start, end);
                assigns.iter().for_each(|assign| {
                    print!("  ");
                    Printer::write_assignment(assign, 0, out)
                        .expect("Printing failed!");
                    println!();
                })
            });
        println!("transitions:");
        self.transitions
            .iter()
            .sorted_by(|(k1, k2, g1), (k3, k4, g2)| {
                k1.cmp(k3).then_with(|| k2.cmp(k4)).then_with(|| g1.cmp(g2))
            })
            .for_each(|(i, f, g)| {
                println!("({})->({})\n  {}", i, f, Printer::guard_str(g));
            })
    }

    /// Returns "runs" of FSM states where transitions happen unconditionally
    fn calculate_runs(&self) -> Vec<Range> {
        let mut unconditional_states = self
            .transitions
            .iter()
            .filter_map(|(s, e, guard)| {
                if *e == s + 1 && guard.is_true() {
                    Some(*s)
                } else {
                    None
                }
            })
            .sorted();

        let mut ranges: Vec<Range> = Vec::new();
        if let Some(mut cur_s) = unconditional_states.next() {
            let mut start_s = cur_s;

            // Extract the next state
            for nxt_s in unconditional_states {
                eprintln!("start: {}, cur: {}, nxt: {}", start_s, cur_s, nxt_s);
                if nxt_s != cur_s + 1 {
                    if cur_s - start_s > 1 {
                        ranges.push((start_s, cur_s));
                    }
                    start_s = cur_s;
                }
                cur_s = nxt_s
            }
            if cur_s - start_s > 1 {
                ranges.push((start_s, cur_s));
            }
        }

        eprintln!("{:?}", ranges);

        ranges
    }

    fn realize_schedule(self) -> RRC<ir::Group> {
        self.calculate_runs();
        let final_state = self.last_state();
        let builder = self.builder;
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
                    let lb_const = builder.add_constant(lb, fsm_size);
                    let ub_const = builder.add_constant(ub, fsm_size);
                    let state_guard;
                    if lb == 0 {
                        state_guard =
                            guard!(fsm["out"]).lt(guard!(ub_const["out"]));
                    } else {
                        state_guard =
                            guard!(fsm["out"]).ge(guard!(lb_const["out"])).and(
                                guard!(fsm["out"]).lt(guard!(ub_const["out"])),
                            );
                    };
                    assigns.iter_mut().for_each(|assign| {
                        assign.guard.update(|g| g.and(state_guard.clone()))
                    });
                    assigns
                }),
        );

        // Transition assignments.
        group.borrow_mut().assignments.extend(
            self.transitions
                .into_iter()
                .sorted_by_key(|(start, _, _)| *start)
                .flat_map(|(start, end, guard)| {
                    structure!(builder;
                        let start_const = constant(start, fsm_size);
                        let end_const = constant(end, fsm_size);
                    );

                    let end_borrow = end_const.borrow();
                    let transition_guard = guard!(fsm["out"])
                        .eq(guard!(start_const["out"]))
                        .and(guard);

                    vec![
                        builder.build_assignment(
                            fsm.borrow().get("in"),
                            end_borrow.get("out"),
                            transition_guard.clone(),
                        ),
                        builder.build_assignment(
                            fsm.borrow().get("write_en"),
                            signal_on.borrow().get("out"),
                            transition_guard,
                        ),
                    ]
                }),
        );

        // Done condition for group.
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

impl Schedule<'_> {
    fn calculate_states(
        &mut self,
        con: &ir::Control,
        // The current state
        cur_state: u64,
        // Additional guard for this condition.
        pre_guard: &ir::Guard,
    ) -> CalyxResult<Vec<PredEdge>> {
        match con {
        ir::Control::Enable(e) => {
            self.enable_calculate_states(e, cur_state, pre_guard)
        }
        ir::Control::Seq(s) => {
            self.seq_calculate_states(s, cur_state, pre_guard)
        }
        ir::Control::Par(p) => {
            self.par_calculate_states(p, cur_state, pre_guard)
        }
        ir::Control::If(i) => {
            self.if_calculate_states(i, cur_state, pre_guard)
        }
        ir::Control::While(w) => {
            self.while_calculate_states(w, cur_state, pre_guard)
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
        self.transitions
            .extend(preds.iter().map(|(s, g)| (s - 1, new_state, g.clone())));

        // Return the new state.
        new_state
    }

    fn seq_calculate_states(
        &mut self,
        con: &ir::Seq,
        cur_state: u64,
        pre_guard: &ir::Guard,
    ) -> CalyxResult<Vec<PredEdge>> {
        let mut preds = vec![];
        let default_pred = (cur_state, pre_guard.clone());
        for stmt in &con.stmts {
            // Add transition(s) from last state to the new state.
            let new_state = self.seq_add_transitions(&preds, &default_pred);

            // Recurse into statement and save new predecessors.
            preds = self.calculate_states(stmt, new_state, pre_guard)?;
        }

        // Add final transition(s) from the last statement.
        self.seq_add_transitions(&preds, &default_pred);

        Ok(preds)
    }

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

    fn if_calculate_states(
        &mut self,
        con: &ir::If,
        cur_state: u64,
        pre_guard: &ir::Guard,
    ) -> CalyxResult<Vec<PredEdge>> {
        if con.cond.is_some() {
            return Err(Error::malformed_structure(
                format!("{}: Found group `{}` in with position of if. This should have compiled away.",
                        TopDownStaticTiming::name(),
                        con.cond.as_ref().unwrap().borrow().name()))
            .with_pos(&con.attributes));
        }

        let port_guard: ir::Guard = Rc::clone(&con.port).into();
        let mut preds = vec![];

        // Then branch.
        preds.extend(self.calculate_states(
            &con.tbranch,
            cur_state,
            &pre_guard.clone().and(port_guard.clone()),
        )?);

        // Else branch.
        preds.extend(self.calculate_states(
            &con.fbranch,
            cur_state,
            &pre_guard.clone().and(port_guard.not()),
        )?);

        Ok(preds)
    }

    fn while_calculate_states(
        &mut self,
        con: &ir::While,
        cur_state: u64,
        pre_guard: &ir::Guard,
    ) -> CalyxResult<Vec<PredEdge>> {
        if con.cond.is_some() {
            return Err(Error::malformed_structure(
                format!("{}: Found group `{}` in with position of while. This should have compiled away.",
                        TopDownStaticTiming::name(),
                        con.cond.as_ref().unwrap().borrow().name()))
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

    /// Compiled to:
    /// ```
    /// group[go] = (fsm >= cur_start & fsm < cur_state + static) & pre_guard ? 1'd1;
    /// ```
    fn enable_calculate_states(
        &mut self,
        con: &ir::Enable,
        // The current state
        cur_state: u64,
        // Additional guard for this condition.
        pre_guard: &ir::Guard,
    ) -> CalyxResult<Vec<PredEdge>> {
        let time_option = con.attributes.get("static");
        if time_option.is_none() {
            return Err(Error::pass_assumption(
            TopDownStaticTiming::name().to_string(),
            "enable is missing @static annotation. This happens when the enclosing control program has a @static annotation but the enable is missing one.".to_string(),
        )
        .with_pos(&con.attributes));
        }
        let time = time_option.unwrap();

        let range = (cur_state, cur_state + time);
        let group = &con.group;
        structure!(self.builder;
            let signal_on = constant(1, 1);
        );
        let mut assigns = build_assignments!(self.builder;
            group["go"] = pre_guard ? signal_on["out"];
        );

        // Enable when in range of group's latency.
        self.enables.entry(range).or_default().append(&mut assigns);

        // Add any necessary internal transitions. In the case time is 1 and there
        // is a single transition, it is taken care of by the parent.
        let starts = cur_state..cur_state + time - 1;
        let ends = cur_state + 1..cur_state + time;
        self.transitions
            .extend(starts.zip(ends).map(|(s, e)| (s, e, pre_guard.clone())));

        Ok(vec![(cur_state + time, pre_guard.clone())])
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
        "top-down-st"
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
        schedule.seq_calculate_states(con, 0, &ir::Guard::True)?;

        // Dump FSM if requested.
        if self.dump_fsm {
            schedule.display();
        }

        // Realize the schedule in a replacement control group.
        let group = schedule.realize_schedule();

        Ok(Action::Change(ir::Control::enable(group)))
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

        Ok(Action::Change(ir::Control::enable(group)))
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

        Ok(Action::Change(ir::Control::enable(group)))
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
        schedule.if_calculate_states(con, 0, &ir::Guard::True)?;

        // Dump FSM if requested.
        if self.dump_fsm {
            schedule.display();
        }

        // Realize the schedule in a replacement control group.
        let group = schedule.realize_schedule();

        Ok(Action::Change(ir::Control::enable(group)))
    }
}
