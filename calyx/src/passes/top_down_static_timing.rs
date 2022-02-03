use super::math_utilities::get_bit_width_from;
use crate::errors::{CalyxResult, Error};
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures, Printer, RRC,
};
use crate::{build_assignments, guard, structure};
use itertools::Itertools;
use std::collections::HashMap;
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
#[derive(Default)]
struct Schedule {
    enables: HashMap<Range, Vec<ir::Assignment>>,
    transitions: Vec<(u64, u64, ir::Guard)>,
}

impl Schedule {
    // TODO: this should be based on transitions when we have them.
    fn last_state(&self) -> u64 {
        self.enables.iter().map(|(k, _)| k.1).max().unwrap()
    }

    #[allow(dead_code)]
    fn display(&self) {
        self.enables
            .iter()
            .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .for_each(|(state, assigns)| {
                eprint!("({}, {}): ", state.0, state.1);
                assigns.iter().for_each(|assign| {
                    Printer::write_assignment(
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
                eprintln!("({}, {}): {}", i, f, Printer::guard_str(&g));
            })
    }

    fn realize_schedule(self, builder: &mut ir::Builder) -> RRC<ir::Group> {
        let group = builder.add_group("tdst");
        let final_state = self.last_state();
        let fsm_size = get_bit_width_from(final_state + 1);

        structure!(builder;
            let fsm = prim std_reg(fsm_size);
        );

        group.borrow_mut().assignments.extend(
            self.enables
                .into_iter()
                .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
                .flat_map(|((lb, ub), mut assigns)| {
                    let lb_const = builder.add_constant(lb, fsm_size);
                    let ub_const = builder.add_constant(ub, fsm_size);
                    let state_guard = guard!(fsm["out"])
                        .ge(guard!(lb_const["out"]))
                        .and(guard!(fsm["out"]).lt(guard!(ub_const["out"])));
                    assigns.iter_mut().for_each(|assign| {
                        assign.guard.update(|g| g.and(state_guard.clone()))
                    });
                    assigns
                }),
        );

        group
    }
}

/// Represents an edge from a predeccesor to the current control node.
/// The `u64` represents the FSM state of the predeccesor and the guard needs
/// to be true for the predeccesor to transition to the current state.
type PredEdge = (u64, ir::Guard);

fn calculate_states(
    con: &ir::Control,
    // The current state
    cur_state: u64,
    // Additional guard for this condition.
    pre_guard: &ir::Guard,
    // Current schedule.
    schedule: &mut Schedule,
    // Component builder
    builder: &mut ir::Builder,
) -> CalyxResult<Vec<PredEdge>> {
    match con {
        ir::Control::Enable(e) => {
            enable_calculate_states(e, cur_state, pre_guard, schedule, builder)
        }
        ir::Control::Seq(s) => {
            seq_calculate_states(s, cur_state, pre_guard, schedule, builder)
        }
        ir::Control::Par(p) => {
            par_calculate_states(p, cur_state, pre_guard, schedule, builder)
        }
        ir::Control::If(i) => {
            if_calculate_states(i, cur_state, pre_guard, schedule, builder)
        }
        ir::Control::While(w) => {
            while_calculate_states(w, cur_state, pre_guard, schedule, builder)
        }
        _ => panic!("Not yet implemented!"),
    }
}

fn seq_calculate_states(
    con: &ir::Seq,
    cur_state: u64,
    pre_guard: &ir::Guard,
    schedule: &mut Schedule,
    builder: &mut ir::Builder,
) -> CalyxResult<Vec<PredEdge>> {
    let mut preds = vec![];
    let mut new_state = cur_state;
    for stmt in &con.stmts {
        // Compute the new start state from the latest predecessor.
        new_state = preds
            .iter()
            .max_by_key(|(state, _)| state)
            .unwrap_or(&(cur_state, pre_guard.clone()))
            .0;

        // Add transitions from each predecessor to the new state.
        schedule.transitions.extend(
            preds
                .iter()
                .map(|(s, g)| (s.clone() - 1, new_state, g.clone())),
        );

        // Recurse into statement and save new predecessors.
        match calculate_states(stmt, new_state, pre_guard, schedule, builder) {
            Ok(inner_preds) => {
                preds = inner_preds;
            }
            Err(e) => return Err(e),
        }
    }

    // Add transition out of last state in the sequence.
    schedule
        .transitions
        .extend(preds.iter().map(|(s, g)| (new_state, s.clone(), g.clone())));

    Ok(preds)
}

fn par_calculate_states(
    con: &ir::Par,
    cur_state: u64,
    pre_guard: &ir::Guard,
    schedule: &mut Schedule,
    builder: &mut ir::Builder,
) -> CalyxResult<Vec<PredEdge>> {
    let cur = cur_state;
    let mut preds = vec![];
    for stmt in &con.stmts {
        match calculate_states(stmt, cur, pre_guard, schedule, builder) {
            Ok(inner_preds) => {
                preds.extend(inner_preds);
            }
            Err(e) => return Err(e),
        }
    }
    Ok(preds)
}

fn if_calculate_states(
    con: &ir::If,
    cur_state: u64,
    pre_guard: &ir::Guard,
    schedule: &mut Schedule,
    builder: &mut ir::Builder,
) -> CalyxResult<Vec<PredEdge>> {
    if con.cond.is_some() {
        return Err(Error::malformed_structure(format!("{}: Found group `{}` in with position of if. This should have compiled away.", TopDownStaticTiming::name(), con.cond.as_ref().unwrap().borrow().name())));
    }

    let port_guard: ir::Guard = Rc::clone(&con.port).into();
    let mut preds = vec![];

    // If branch.
    match calculate_states(
        &con.tbranch,
        cur_state,
        &port_guard,
        schedule,
        builder,
    ) {
        Ok(inner_preds) => preds.extend(inner_preds),
        Err(e) => return Err(e),
    }

    // Else branch.
    match calculate_states(
        &con.fbranch,
        cur_state,
        &port_guard.not(),
        schedule,
        builder,
    ) {
        Ok(inner_preds) => preds.extend(inner_preds),
        Err(e) => return Err(e),
    }

    Ok(preds)
}

fn while_calculate_states(
    con: &ir::While,
    cur_state: u64,
    pre_guar: &ir::Guard,
    schedule: &mut Schedule,
    builder: &mut ir::Builder,
) -> CalyxResult<Vec<PredEdge>> {
    todo!();
}

/// Compiled to:
/// ```
/// group[go] = (fsm >= cur_start & fsm < cur_state + static) & pre_guard ? 1'd1;
/// ```
fn enable_calculate_states(
    con: &ir::Enable,
    // The current state
    cur_state: u64,
    // Additional guard for this condition.
    pre_guard: &ir::Guard,
    // Current schedule.
    schedule: &mut Schedule,
    // Component builder
    builder: &mut ir::Builder,
) -> CalyxResult<Vec<PredEdge>> {
    let time = con
        .attributes
        .get("static")
        .expect("`static` annotation missing");
    let range = (cur_state, cur_state + time);
    let group = &con.group;
    structure!(builder;
        let signal_on = constant(1, 1);
    );
    let mut assigns = build_assignments!(builder;
        group["go"] = pre_guard ? signal_on["out"];
    );

    // Enable when in range of group's latency.
    schedule
        .enables
        .entry(range)
        .or_default()
        .append(&mut assigns);

    // Transition to the next state latency times.
    let starts = cur_state..cur_state + time - 1;
    let ends = cur_state + 1..cur_state + time;
    schedule
        .transitions
        .extend(starts.zip(ends).map(|(s, e)| (s, e, pre_guard.clone())));

    Ok(vec![(cur_state + time, pre_guard.clone())])
}

#[derive(Default)]
pub struct TopDownStaticTiming;

impl Named for TopDownStaticTiming {
    fn name() -> &'static str {
        "top-down-st"
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
        // Do not try to compile an enable or empty control
        if matches!(
            *comp.control.borrow(),
            ir::Control::Enable(..) | ir::Control::Empty(..)
        ) {
            return Ok(Action::Stop);
        }

        let control = Rc::clone(&comp.control);
        let mut schedule = Schedule::default();
        let mut builder = ir::Builder::new(comp, sigs);

        let result = calculate_states(
            &control.borrow(),
            0,
            &ir::Guard::True,
            &mut schedule,
            &mut builder,
        );

        if result.is_err() {
            return Err(result.unwrap_err());
        }

        schedule.display();

        let group = schedule.realize_schedule(&mut builder);

        Ok(Action::Change(ir::Control::enable(group)))
    }
}
