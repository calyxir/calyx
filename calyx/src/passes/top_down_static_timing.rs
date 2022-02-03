use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures, Printer,
};
use crate::{build_assignments, structure};
use itertools::Itertools;
use std::collections::HashMap;
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
}

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
) -> u64 {
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
) -> u64 {
    let mut cur = cur_state;
    for stmt in &con.stmts {
        cur = calculate_states(stmt, cur, pre_guard, schedule, builder)
    }
    cur
}

fn par_calculate_states(
    con: &ir::Par,
    cur_state: u64,
    pre_guard: &ir::Guard,
    schedule: &mut Schedule,
    builder: &mut ir::Builder,
) -> u64 {
    let cur = cur_state;
    let mut max = 0;
    for stmt in &con.stmts {
        let next = calculate_states(stmt, cur, pre_guard, schedule, builder);
        if next > max {
            max = next;
        }
    }
    max
}

fn if_calculate_states(
    con: &ir::If,
    cur_state: u64,
    pre_guard: &ir::Guard,
    schedule: &mut Schedule,
    builder: &mut ir::Builder,
) -> u64 {
    structure!(builder;
        let signal_on = constant(1, 1);
        let signal_off = constant(0, 1);
        let st_cs_if = prim std_reg(1);
    );

    // If the condition is defined using a combinational group, make it
    // take one cycle.

    // Compute the value in the condition and save its value in cs_if

    todo!()
}

fn while_calculate_states(
    con: &ir::While,
    cur_state: u64,
    pre_guar: &ir::Guard,
    schedule: &mut Schedule,
    builder: &mut ir::Builder,
) -> u64 {
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
) -> u64 {
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
    schedule
        .enables
        .entry(range)
        .or_default()
        .append(&mut assigns);
    cur_state + time
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

        calculate_states(
            &control.borrow(),
            0,
            &ir::Guard::True,
            &mut schedule,
            &mut builder,
        );

        schedule.display();

        Ok(Action::Continue)
    }
}
