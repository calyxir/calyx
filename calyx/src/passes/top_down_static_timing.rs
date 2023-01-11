use super::math_utilities::get_bit_width_from;
use crate::errors::{CalyxResult, Error};
use crate::ir::traversal::ConstructVisitor;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures, Printer, RRC,
};
use crate::ir::{Attributes, CloneName, GetAttributes};
use crate::{build_assignments, guard, passes, structure};
use itertools::Itertools;
use std::collections::HashMap;
use std::ops::Not;
use std::rc::Rc;
use std::{cmp, iter};

const ID: &str = "ST_ID";
const LOOP: &str = "LOOP";
const START: &str = "START";
const END: &str = "END";

/// A range of FSM states.
type Range = (u64, u64);

/// Preprocessing phase applied before the generation of the FSM:
/// 1. Allocates unique start states for each enable in the control program.
/// 2. Balances the branches of all `if` so that they take the same amount of
///    time to execute.
/// 3. De-nest directly nested bounded `while` loops.
///
/// The FSM generation process requires the above noramalization to be applied.
struct States {
    /// Current state
    cur_st: u64,
    /// Mapping for loop indices
    indices: Vec<RRC<ir::Cell>>,
}
impl Default for States {
    fn default() -> Self {
        Self {
            /// 0 is a special start state allocated to the start of the
            /// program so we start with the state 1.
            cur_st: 1,
            indices: vec![],
        }
    }
}

impl States {
    /// Take a control program and ensure that its execution time is at least `time`.
    fn extend_control(
        &self,
        con: &mut Box<ir::Control>,
        time: u64,
        balance: &ir::Enable,
    ) {
        let cur_time = con.get_attribute("static").unwrap();

        if cur_time < time {
            let bal = ir::Control::Enable(ir::Cloner::enable(balance));
            let inner = *std::mem::replace(con, Box::new(ir::Control::empty()));
            let extra = (0..time - cur_time).map(|_| ir::Cloner::control(&bal));
            let mut seq =
                ir::Control::seq(iter::once(inner).chain(extra).collect());
            seq.get_mut_attributes().insert("static", time);
            *con = Box::new(seq);
        }
    }

    /// Transform nested bounded loops into a singly nested loop:
    /// ```
    /// @bound(m) while r0.out {
    ///   @bound(n) while r1.out {
    ///     @bound(l) while r2.out { body }
    ///   }
    /// }
    /// ```
    /// Into:
    /// ```
    /// @bound(m*n*l) while r0.out { body }
    /// ```
    ///
    /// Note that after this transformation, it is no longer correct to lower
    /// the loop using TDCC since we've ignored the loop entry conditions.
    fn denest_loop(wh: &mut ir::While) {
        let mut body =
            std::mem::replace(&mut wh.body, Box::new(ir::Control::empty()));
        let mut bound = wh.attributes["bound"];
        let mut body_time = body.get_attribute("static").unwrap();

        while let ir::Control::While(inner) = *body {
            bound *= inner.attributes["bound"];
            body = inner.body;
            body_time = body.get_attribute("static").unwrap();
        }
        wh.body = body;
        wh.attributes["bound"] = bound;
        wh.attributes["static"] = body_time * bound;
    }

    fn compute_states(
        &mut self,
        con: &mut ir::Control,
        builder: &mut ir::Builder,
        balance: &ir::Enable,
    ) {
        match con {
            ir::Control::Enable(en) => {
                en.attributes[ID] = self.cur_st;
                let time = en.attributes["static"];
                self.cur_st += time;
            }
            ir::Control::Seq(seq) => {
                self.compute_seq(seq, builder, balance);
            }
            ir::Control::If(if_) => {
                self.compute_if(if_, builder, balance);
            }
            ir::Control::While(wh) => {
                self.compute_while(wh, builder, balance);
            }
            ir::Control::Par(_) => todo!(),
            ir::Control::Invoke(_) => {
                unreachable!(
                    "Invoke statements should have been compiled away."
                )
            }
            ir::Control::Empty(_) => {
                unreachable!("Empty blocks should have been compiled away")
            }
        }
    }

    fn compute_while(
        &mut self,
        wh: &mut ir::While,
        builder: &mut ir::Builder,
        balance: &ir::Enable,
    ) {
        // Normalize the loop
        Self::denest_loop(wh);

        // Compute START, END, and LOOP index attributes
        wh.attributes[START] = self.cur_st;
        let body_time = wh.attributes["static"];
        // Instantiate the indexing variable for this while loop
        let size = get_bit_width_from(body_time + 1);
        structure!(builder;
            let idx = prim std_reg(size);
        );
        self.indices.push(idx);
        let idx_pos = self.indices.len() - 1;
        // Add attribute to track the loop counter
        wh.attributes[LOOP] = idx_pos as u64;
        self.compute_states(&mut wh.body, builder, balance);
        // Mark the end state of the body
        wh.attributes[END] = self.cur_st;
    }
    fn new_while(
        wh: &mut ir::While,
        builder: &mut ir::Builder,
        balance: &ir::Enable,
    ) -> Self {
        let mut states = Self::default();
        states.compute_while(wh, builder, balance);
        states
    }

    fn compute_if(
        &mut self,
        if_: &mut ir::If,
        builder: &mut ir::Builder,
        balance: &ir::Enable,
    ) {
        // Balance the branches
        let ttime = if_.tbranch.get_attribute("static").unwrap();
        let ftime = if_.fbranch.get_attribute("static").unwrap();
        let max_time = cmp::max(ttime, ftime);
        self.extend_control(&mut if_.tbranch, max_time, balance);
        self.extend_control(&mut if_.fbranch, max_time, balance);
        // Compute states
        self.compute_states(&mut if_.tbranch, builder, balance);
        self.compute_states(&mut if_.fbranch, builder, balance);
    }
    fn new_if(
        if_: &mut ir::If,
        builder: &mut ir::Builder,
        balance: &ir::Enable,
    ) -> Self {
        let mut states = Self::default();
        states.compute_if(if_, builder, balance);
        states
    }

    fn compute_seq(
        &mut self,
        seq: &mut ir::Seq,
        builder: &mut ir::Builder,
        balance: &ir::Enable,
    ) {
        for stmt in &mut seq.stmts {
            self.compute_states(stmt, builder, balance);
        }
    }
    fn new_seq(
        seq: &mut ir::Seq,
        builder: &mut ir::Builder,
        balance: &ir::Enable,
    ) -> Self {
        let mut states = Self::default();
        states.compute_seq(seq, builder, balance);
        states
    }

    /// Computes the outgoing edges from the control programs.
    /// **Requires**: `con` is a sub-program of the control program used to
    /// construct this [States] instance.
    fn control_exits(
        &self,
        con: &ir::Control,
        builder: &mut ir::Builder,
        exits: &mut Vec<PredEdge>,
    ) {
        match con {
            ir::Control::Enable(en) => {
                let st = en.attributes[ID] + en.attributes["static"] - 1;
                exits.push((st, ir::Guard::True));
            }
            ir::Control::Seq(s) => {
                if let Some(stmt) = s.stmts.last() {
                    self.control_exits(stmt, builder, exits);
                }
            }
            ir::Control::If(if_) => {
                let ir::If {
                    tbranch, fbranch, ..
                } = if_;
                self.control_exits(tbranch, builder, exits);
                self.control_exits(fbranch, builder, exits);
            }
            ir::Control::While(wh) => {
                let ir::While { body, .. } = wh;
                // Compute the exit conditions for the loop body
                let mut loop_exits = Vec::new();
                self.control_exits(body, builder, &mut loop_exits);

                // Guard the exit edges for the body with the loop exit condition
                let (idx, bound) = self.loop_bounds(wh, builder);
                let guard = guard!(idx["out"]).eq(guard!(bound["out"]));
                exits.extend(
                    loop_exits
                        .into_iter()
                        .map(|(st, g)| (st, g & guard.clone())),
                );
            }
            ir::Control::Invoke(_) => {
                unreachable!("Invoke should have been compiled away")
            }
            ir::Control::Empty(_) => {
                unreachable!("Empty block in control_exits")
            }
            ir::Control::Par(_) => unreachable!("Par blocks in control_exits"),
        }
    }

    /// Generate the guard condition for exiting the given loop.
    /// **Requires**: The loop is a sub-program of the control program used to
    /// generate this [States] instance.
    fn loop_bounds(
        &self,
        wh: &ir::While,
        builder: &mut ir::Builder,
    ) -> (RRC<ir::Cell>, RRC<ir::Cell>) {
        let max_count =
            wh.body.get_attribute("static").unwrap() * wh.attributes["bound"];
        let size = get_bit_width_from(max_count + 1);
        structure!(builder;
            let max = constant(max_count, size);
        );
        let idx_pos = wh.attributes[LOOP] as usize;
        let idx = Rc::clone(&self.indices[idx_pos]);
        (idx, max)
    }
}

/// A schedule keeps track of two things:
/// 1. `enables`: Specifies which groups are active during a range of
///     FSM states.
/// 2. `transitions`: Transitions for the FSM registers. A static FSM normally
///    transitions from `state` to `state + 1`. However, special transitions
///    are needed for loops, conditionals, and reseting the FSM.
struct Schedule<'a, 'b: 'a> {
    /// Enable assignments in a particular range
    enables: HashMap<Range, Vec<ir::Assignment>>,
    /// Transition from one state to another when a guard is true
    transitions: Vec<(u64, u64, ir::Guard)>,
    // Builder for the associated component
    builder: &'b mut ir::Builder<'a>,
    states: States,
}

impl<'a, 'b> Schedule<'a, 'b> {
    fn new(builder: &'a mut ir::Builder<'a>, states: States) -> Self {
        Self {
            enables: HashMap::default(),
            transitions: Vec::new(),
            builder,
            states,
        }
    }

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
        self.transitions.push((start, end, guard));
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
           let fsm = prim std_reg(fsm_size);
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
        let (st, g) = out_edges.pop().expect("No outgoing edges");
        let c = builder.add_constant(st, fsm_size);
        let mut done_guard = guard!(fsm["out"]).eq(guard!(c["out"])) & g;
        for (st, g) in out_edges {
            let stc = builder.add_constant(st, fsm_size);
            let st_guard = guard!(fsm["out"]).eq(guard!(stc["out"]));
            done_guard |= st_guard & g;
        }
        let done_assign = build_assignments!(builder;
            group["done"] = done_guard ? signal_on["out"];
        );
        group.borrow_mut().assignments.extend(done_assign);

        // Cleanup: Add a transition from last state to the first state.
        let mut reset_fsm = build_assignments!(builder;
            fsm["in"] = done_guard ? first_state["out"];
            fsm["write_en"] = done_guard ? signal_on["out"];
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
            ir::Control::Par(_) => {
                unimplemented!("Static par")
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
                        super::RemoveCombGroups::name()))
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
                super::RemoveCombGroups::name())
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
            // Ensure that reset assignments are active when exiting the loop
            self.add_enables(st, st + 1, iter::once(reset_activate.clone()));
        }

        let exits = body_preds
            .into_iter()
            .map(|(st, g)| (st, g & exit.clone()))
            .collect_vec();

        Ok(exits)
    }
}

impl<'a, 'b> Schedule<'a, 'b>
where
    'b: 'a,
{
    fn realize_seq(
        seq: &mut ir::Seq,
        builder: &'b mut ir::Builder<'a>,
        balance: &ir::Enable,
        dump_fsm: bool,
    ) -> CalyxResult<RRC<ir::Group>> {
        let states = States::new_seq(seq, builder, balance);
        let mut sch = Self::new(builder, states);
        let edges =
            sch.seq_calculate_states(seq, vec![(0, ir::Guard::True)])?;
        Ok(sch.realize_schedule(edges, dump_fsm))
    }

    fn realize_if(
        if_: &mut ir::If,
        builder: &'b mut ir::Builder<'a>,
        balance: &ir::Enable,
        dump_fsm: bool,
    ) -> CalyxResult<RRC<ir::Group>> {
        let states = States::new_if(if_, builder, balance);
        let mut sch = Self::new(builder, states);
        let edges = sch.if_calculate_states(if_, vec![(0, ir::Guard::True)])?;
        Ok(sch.realize_schedule(edges, dump_fsm))
    }

    fn realize_while(
        wh: &mut ir::While,
        builder: &'b mut ir::Builder<'a>,
        balance: &ir::Enable,
        dump_fsm: bool,
    ) -> CalyxResult<RRC<ir::Group>> {
        let states = States::new_while(wh, builder, balance);
        let mut sch = Self::new(builder, states);
        let edges =
            sch.while_calculate_states(wh, vec![(0, ir::Guard::True)])?;
        Ok(sch.realize_schedule(edges, dump_fsm))
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
    /// Make sure that the program is fully compiled by this pass
    force: bool,
    /// Control operator to enable the balancing group.
    balance: Option<ir::Enable>,
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
            balance: None,
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

impl Visitor for TopDownStaticTiming {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Add dummy group that is used for balancing branches
        let mut builder = ir::Builder::new(comp, sigs);
        let balance = builder.add_group("balance");
        balance.borrow_mut().attributes.insert("static", 1);
        let mut enable = ir::Enable {
            group: balance,
            attributes: Attributes::default(),
        };
        enable.attributes.insert("static", 1);
        self.balance = Some(enable);

        Ok(Action::Continue)
    }

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
        let balance = self.balance.as_ref().unwrap();
        // Realize the schedule in a replacement control group.
        let group =
            Schedule::realize_seq(con, &mut builder, balance, self.dump_fsm)?;

        Ok(Action::change(ir::Control::enable(group)))
    }

    /*
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
    */

    fn start_while(
        &mut self,
        con: &mut ir::While,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let time_option = con.attributes.get("static");
        let bound_option = con.attributes.get("bound");

        // If sub-tree is not static, skip this node.
        if time_option.is_none() || bound_option.is_none() {
            return Ok(Action::Continue);
        }

        // Compile control program and save schedule.
        let mut builder = ir::Builder::new(comp, sigs);
        let balance = self.balance.as_ref().unwrap();
        // Realize the schedule in a replacement control group.
        let group =
            Schedule::realize_while(con, &mut builder, balance, self.dump_fsm)?;

        let en = ir::Control::enable(group);
        Ok(Action::change(en))
    }

    fn start_if(
        &mut self,
        con: &mut ir::If,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // If sub-tree is not static, skip this node.
        if con.attributes.get("static").is_none() {
            return Ok(Action::Continue);
        }

        // Compile control program and save schedule.
        let mut builder = ir::Builder::new(comp, sigs);
        let balance = self.balance.as_ref().unwrap();
        // Realize the schedule in a replacement control group.
        let group =
            Schedule::realize_if(con, &mut builder, balance, self.dump_fsm)?;

        Ok(Action::change(ir::Control::enable(group)))
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // If the force flag is set, make sure that we only have one group remaining
        let con = &*comp.control.borrow();
        if self.force && !matches!(con, ir::Control::Enable(_)) {
            return Err(Error::pass_assumption(
                Self::name(),
                "`force` flag was set but the final control program is not an enable"
            ).with_pos(con));
        }
        Ok(Action::Continue)
    }
}
