use super::math_utilities::get_bit_width_from;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures, RRC,
};
use crate::{build_assignments, guard, structure};
use ir::IRPrinter;
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use petgraph::{algo::connected_components, graph::DiGraph};
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
            connected_components(&graph) == 1,
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
                    eprintln!("");
                })
            });
        eprintln!("------------");
        self.transitions
            .iter()
            .sorted_by(|(k1, _, _), (k2, _, _)| k1.cmp(k2))
            .for_each(|(i, f, g)| {
                eprintln!("({}, {}): {}", i, f, IRPrinter::guard_str(&g));
            })
    }
}

/// Implement the given schedule using a new FSM.
fn realize_schedule(
    schedule: Schedule,
    builder: &mut ir::Builder,
) -> RRC<ir::Group> {
    schedule.validate();
    let final_state = schedule.last_state();
    let fsm_size =
        get_bit_width_from(final_state + 1 /* represent 0..final_state */);
    structure!(builder;
        let fsm = prim std_reg(fsm_size);
        let signal_on = constant(1, 1);
        let last_state = constant(final_state, fsm_size);
        let first_state = constant(0, fsm_size);
    );

    // The compilation group
    let group = builder.add_group("tdcc", LinkedHashMap::with_capacity(0));

    // Enable assignments
    group.borrow_mut().assignments.extend(
        schedule
            .enables
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
        schedule.transitions.into_iter().flat_map(|(s, e, guard)| {
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
        ir::Control::Enable(ir::Enable { group }) => {
            let done_cond = guard!(group["done"]) & pre_guard.clone();
            let not_done = !guard!(group["done"]) & pre_guard.clone();
            let signal_on = builder.add_constant(1, 1);
            let mut en_go = build_assignments!(builder;
                group["go"] = not_done ? signal_on["out"];
            );
            let nxt_state = cur_state + 1;
            schedule
                .enables
                .entry(cur_state)
                .or_default()
                .append(&mut en_go);

            schedule.transitions.push((cur_state, nxt_state, done_cond));
            nxt_state
        }
        ir::Control::Seq(ir::Seq { stmts }) => {
            let mut cur = cur_state;
            for stmt in stmts {
                cur = calculate_states(stmt, cur, pre_guard, schedule, builder);
            }
            cur
        }
        ir::Control::If(ir::If {
            port,
            cond,
            tbranch,
            fbranch,
        }) => {
            structure!(builder;
                let signal_on = constant(1, 1);
                let signal_off = constant(0, 1);
                let cs_if = prim std_reg(1);
            );

            let mut cond_save_assigns = vec![
                builder.build_assignment(
                    cs_if.borrow().get("in"),
                    Rc::clone(&port),
                    pre_guard.clone(),
                ),
                builder.build_assignment(
                    cs_if.borrow().get("write_en"),
                    signal_on.borrow().get("out"),
                    pre_guard.clone(),
                ),
                builder.build_assignment(
                    cond.borrow().get("go"),
                    signal_on.borrow().get("out"),
                    pre_guard.clone(),
                ),
            ];

            // Compute the condition first
            schedule
                .enables
                .entry(cur_state)
                .or_default()
                .append(&mut cond_save_assigns);

            let after_cond_compute = cur_state + 1;
            schedule.transitions.push((
                cur_state,
                after_cond_compute,
                guard!(cond["done"]),
            ));
            let true_go = guard!(cs_if["out"]) & pre_guard.clone();
            let false_go = !guard!(cs_if["out"]) & pre_guard.clone();
            // Computation for true branch
            let after_true = calculate_states(
                tbranch,
                after_cond_compute,
                &true_go,
                schedule,
                builder,
            );
            // Computation for false branch
            let after_false = calculate_states(
                fbranch,
                after_cond_compute,
                &false_go,
                schedule,
                builder,
            );
            // Transition to a join stage
            let next = std::cmp::max(after_true, after_false) + 1;
            schedule.transitions.push((after_true, next, true_go));
            schedule.transitions.push((after_false, next, false_go));
            // Cleanup
            let mut cleanup = build_assignments!(builder;
                cs_if["in"] = pre_guard ? signal_off["out"];
                cs_if["write_en"] = pre_guard ? signal_on["out"];
            );
            schedule
                .enables
                .entry(next)
                .or_default()
                .append(&mut cleanup);
            next
        }
        ir::Control::While(ir::While { cond, port, body }) => {
            structure!(builder;
                let signal_on = constant(1, 1);
                let signal_off = constant(0, 1);
                let cs_wh = prim std_reg(1);
            );

            let mut cond_save_assigns = vec![
                builder.build_assignment(
                    cs_wh.borrow().get("in"),
                    Rc::clone(&port),
                    pre_guard.clone(),
                ),
                builder.build_assignment(
                    cs_wh.borrow().get("write_en"),
                    signal_on.borrow().get("out"),
                    pre_guard.clone(),
                ),
                builder.build_assignment(
                    cond.borrow().get("go"),
                    signal_on.borrow().get("out"),
                    pre_guard.clone(),
                ),
            ];

            // Compute the condition first
            let after_cond_compute = cur_state + 1;
            schedule
                .enables
                .entry(cur_state)
                .or_default()
                .append(&mut cond_save_assigns);
            schedule.transitions.push((
                cur_state,
                after_cond_compute,
                guard!(cond["done"]),
            ));

            let body_go = guard!(cs_wh["out"]) & pre_guard.clone();
            let wh_done = !guard!(cs_wh["out"]) & pre_guard.clone();

            // Build the FSM for the body
            let nxt = calculate_states(
                &body,
                after_cond_compute,
                &body_go,
                schedule,
                builder,
            );

            // Back edge jump when condition was true
            schedule.transitions.push((nxt, cur_state, body_go));

            // Exit state: Jump to this when the condition is false.
            let exit = nxt + 1;
            schedule
                .transitions
                .push((after_cond_compute, exit, wh_done));

            // Cleanup state registers in exit stage
            let mut cleanup = build_assignments!(builder;
                cs_wh["in"] = pre_guard ? signal_off["out"];
                cs_wh["write_en"] = pre_guard ? signal_on["out"];
            );
            schedule
                .enables
                .entry(exit)
                .or_default()
                .append(&mut cleanup);
            // Condition transitions
            exit
        }
        ir::Control::Par(..) => {
            unreachable!("par should be compiled away!")
        }
        ir::Control::Empty(..) => {
            unreachable!("empty control should have been compiled away!")
        }
        ir::Control::Invoke(..) => {
            unreachable!("invoke should have been compiled away!")
        }
    }
}

/// Top-down FSM generation for compiling away control constructs.
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
        let mut builder = ir::Builder::from(comp, sigs, false);
        // Compilation group
        let par_group =
            builder.add_group("par", LinkedHashMap::with_capacity(0));
        structure!(builder;
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
        );

        let done_regs = s
            .stmts
            .iter()
            .map(|con| {
                let group = match con {
                    // Do not compile enables
                    ir::Control::Enable(ir::Enable { group }) => {
                        Rc::clone(group)
                    }
                    _ => {
                        // Compile complex schedule and return the group.
                        let mut schedule = Schedule::default();
                        calculate_states(
                            &con,
                            0,
                            &ir::Guard::True,
                            &mut schedule,
                            &mut builder,
                        );
                        realize_schedule(schedule, &mut builder)
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
                pd
            })
            .collect::<Vec<_>>();

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
        if matches!(*comp.control.borrow(), ir::Control::Enable(..)) {
            return Ok(Action::Stop);
        }
        let control = Rc::clone(&comp.control);
        let mut builder = ir::Builder::from(comp, sigs, false);
        let mut schedule = Schedule::default();
        calculate_states(
            &control.borrow(),
            0,
            &ir::Guard::True,
            &mut schedule,
            &mut builder,
        );
        //schedule.display();
        let comp_group = realize_schedule(schedule, &mut builder);
        Ok(Action::Change(ir::Control::enable(comp_group)))
    }
}
