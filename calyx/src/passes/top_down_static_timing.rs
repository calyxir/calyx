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

/// Represents an FSM that increments every cycle with some non-incrementing transitions. A
/// schedule keeps track of two things:
/// 1. `enables`: Specifies which groups are active during a range of
///     FSM states.
/// 2. `transitions`: Non-increment transitions for the FSM.
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
        assert!(!self.enables.is_empty(), "Transitions are empty");
        self.enables.keys().map(|(_, e)| *e).max().unwrap()
    }

    fn display(&self) {
        let out = &mut std::io::stdout();

        println!("enables:");
        self.enables
            .iter()
            .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .for_each(|((start, _), assigns)| {
                println!("{}:", start);
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
                println!("  ({}, {}):  {}", i, f, Printer::guard_str(g));
            });

        // Unconditional +1 transitions
        let last_state = self.last_state();
        println!("final_state: {}", last_state);
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

        // Condition for incrementing transitions. In these states, do not transition using
        // increment.
        let max_val = final_state;
        let mv = builder.add_constant(max_val, fsm_size);
        let lt_guard = guard!(fsm["out"]).lt(guard!(mv["out"]));
        let not_transition = self
            .transitions
            .iter()
            .map(|(s, _, _)| {
                let start_const =
                    builder.add_constant(*s, fsm_size).borrow().get("out");
                guard!(fsm["out"]).neq(start_const.into())
            })
            .fold(lt_guard, |acc, g| acc & g);

        structure!(builder;
            let fsm_incr = prim std_add(fsm_size);
            let one = constant(1, fsm_size);
        );
        let uncond_incr = build_assignments!(builder;
            fsm_incr["left"] = ? fsm["out"];
            fsm_incr["right"] = ? one["out"];
            fsm["in"] = not_transition ? fsm_incr["out"];
            fsm["write_en"] = not_transition ? signal_on["out"];
        );
        group.borrow_mut().assignments.extend(uncond_incr);

        // Non-incrementing transitions
        group.borrow_mut().assignments.extend(
            self.transitions
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

impl Schedule<'_> {
    fn calculate_states(
        &mut self,
        con: &ir::Control,
        // The current state
        cur_state: u64,
        // Additional guard for this condition.
        pre_guard: &ir::Guard,
    ) -> CalyxResult<u64> {
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

    fn seq_calculate_states(
        &mut self,
        con: &ir::Seq,
        cur_state: u64,
        pre_guard: &ir::Guard,
    ) -> CalyxResult<u64> {
        // Compute the transitions by passing along the state to each child statement.
        let nxt_st = con.stmts.iter().try_fold(cur_state, |st, stmt| {
            self.calculate_states(stmt, st, pre_guard)
        })?;

        Ok(nxt_st)
    }

    fn par_calculate_states(
        &mut self,
        con: &ir::Par,
        cur_state: u64,
        pre_guard: &ir::Guard,
    ) -> CalyxResult<u64> {
        let max_state_res: CalyxResult<u64> =
            con.stmts.iter().try_fold(u64::MIN, |max, stmt| {
                let st = self.calculate_states(stmt, cur_state, pre_guard)?;
                Ok(if st > max { st } else { max })
            });
        let max_state = max_state_res?;

        // Return a single predecessor for the last state.
        Ok(max_state)
    }

    fn if_calculate_states(
        &mut self,
        con: &ir::If,
        cur_state: u64,
        pre_guard: &ir::Guard,
    ) -> CalyxResult<u64> {
        if con.cond.is_some() {
            return Err(Error::malformed_structure(
                format!("{}: Found group `{}` in with position of if. This should have compiled away.",
                        TopDownStaticTiming::name(),
                        con.cond.as_ref().unwrap().borrow().name()))
            .with_pos(&con.attributes));
        }

        let port_guard: ir::Guard = Rc::clone(&con.port).into();

        // Then branch.
        let tr_end = self.calculate_states(
            &con.tbranch,
            cur_state,
            &pre_guard.clone().and(port_guard.clone()),
        )?;

        // Else branch.
        let fal_end = self.calculate_states(
            &con.fbranch,
            cur_state,
            &pre_guard.clone().and(port_guard.not()),
        )?;

        let max = std::cmp::max(tr_end, fal_end);

        Ok(max)
    }

    fn while_calculate_states(
        &mut self,
        con: &ir::While,
        cur_state: u64,
        pre_guard: &ir::Guard,
    ) -> CalyxResult<u64> {
        if con.cond.is_some() {
            return Err(Error::malformed_structure(
                format!("{}: Found group `{}` in with position of while. This should have compiled away.",
                        TopDownStaticTiming::name(),
                        con.cond.as_ref().unwrap().borrow().name()))
            .with_pos(&con.attributes));
        }

        let mut body_exit = cur_state;

        for _ in 0..*con.attributes.get("bound").unwrap() {
            body_exit = self.calculate_states(
                &con.body,
                body_exit,
                &pre_guard.clone(),
            )?;
        }

        Ok(body_exit)
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
    ) -> CalyxResult<u64> {
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

        Ok(cur_state + time)
    }
}

/// Lowering pass that generates latency-sensitive FSMs when control sub-programs have `@static`
/// annotations. The pass works opportunisitically and attempts to compile all nested static
/// control programs nested within the overall program, replacing them with groups that implement
/// the correct transitions.
///
/// **Balancing**: The pass automatically adds dummy transitions to ensure that branches are
/// balanced, i.e., they take exactly the same number of cycles. In some cases, this may perform
/// worse than the dynamic FSMs generated from `tdcc`.
///
/// **Loops**: `while` control blocks can only be statically compiled when they additionally have a
/// `@bound` annotation which mentions the expected number of times a loop will iterate.
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
