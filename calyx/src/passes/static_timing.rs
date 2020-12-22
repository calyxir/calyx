use crate::frontend::library::ast as lib;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::{build_assignments, guard, structure};
use std::collections::HashMap;
use std::{cmp, rc::Rc};

#[derive(Default)]
/// Optimized lowering for control statements that only contain groups with
/// the "static" attribute.
///
/// Structured as a bottom-up pass. Control constructs not compiled by this
/// pass are compiled by the generic `CompileControl` pass.
pub struct StaticTiming {}

impl Named for StaticTiming {
    fn name() -> &'static str {
        "static-timing"
    }

    fn description() -> &'static str {
        "Opportunistically compile timed groups and generate timing information when possible."
    }
}

/// Function to iterate over a vector of control statements and collect
/// the "static" attribute using the `acc` function.
/// Returns None if any of of the Control statements is a compound statement.
fn accumulate_static_time<F>(stmts: &[ir::Control], acc: F) -> Option<u64>
where
    F: FnMut(u64, u64) -> u64,
{
    let timing: Result<Vec<u64>, ()> = stmts
        .iter()
        .map(|con| {
            if let ir::Control::Enable(data) = con {
                data.group
                    .borrow()
                    .attributes
                    .get("static")
                    .copied()
                    .ok_or(())
            } else {
                Err(())
            }
        })
        .collect();

    timing.ok().map(|ts| ts.into_iter().fold(0, acc))
}

impl Visitor for StaticTiming {
    fn finish_while(
        &mut self,
        while_s: &mut ir::While,
        comp: &mut ir::Component,
        ctx: &lib::LibrarySignatures,
    ) -> VisResult {
        // let st = &mut comp.structure;

        if let ir::Control::Enable(data) = &*while_s.body {
            let cond = &while_s.cond;
            let port = &while_s.port;
            let body = &data.group;
            let mut builder = ir::Builder::from(comp, ctx, false);

            // FSM Encoding:
            //   0:   init state. we haven't started loop iterations
            //        and haven't checked the loop body
            //   1-n: body compute states. cond was true. compute the body
            //   n+1: loop exit. we've finished running the body and the condition is false.
            // Transitions:
            //   0 -> 1:   when cond == true
            //   0 -> n+1: when cond == false
            //   i -> i+1: when i != 0 & i != n
            //   n -> 1:   when cond == true
            //   n -> n+1: when cond == false

            // The group is statically compilable with combinational condition.
            if let (Some(&ctime), Some(&btime)) = (
                cond.borrow().attributes.get("static"),
                body.borrow().attributes.get("static"),
            ) {
                let while_group =
                    builder.add_group("static_while", HashMap::new());

                // take at least one cycle for computing the body and conditiona
                let ctime = cmp::max(ctime, 1);
                let btime = cmp::max(btime, 1);

                let fsm_size = 32;
                structure!(builder;
                    let fsm = prim std_reg(fsm_size);
                    let cond_stored = prim std_reg(1);
                    let fsm_reset_val = constant(0, fsm_size);
                    let fsm_one = constant(1, fsm_size);
                    let incr = prim std_add(fsm_size);

                    let signal_on = constant(1, 1);

                    let cond_time_const = constant(ctime, fsm_size);
                    let body_end_const = constant(ctime + btime, fsm_size);
                );

                // Cond is computed on this cycle.
                let cond_computed =
                    guard!(fsm["out"]).lt(guard!(cond_time_const["out"]));

                let body_done =
                    guard!(fsm["out"]).eq(guard!(body_end_const["out"]));
                // Should we increment the FSM this cycle.
                let fsm_incr = !body_done.clone();

                // Compute the cond group
                let cond_go =
                    guard!(fsm["out"]).lt(guard!(cond_time_const["out"]));

                let body_go = guard!(cond_stored["out"])
                    & !cond_go.clone()
                    & guard!(fsm["out"]).lt(guard!(body_end_const["out"]));

                let done = guard!(fsm["out"])
                    .eq(guard!(cond_time_const["out"]))
                    & !guard!(cond_stored["out"]);

                let mut assignments = build_assignments!(
                    builder;
                    // Increment the FSM when needed
                    incr["left"] = ? fsm["out"];
                    incr["right"] = ? fsm_one["out"];
                    fsm["in"] = fsm_incr ? incr["out"];
                    fsm["write_en"] = fsm_incr ? signal_on["out"];

                    // Compute the cond group and save the result
                    cond["go"] = cond_go ? signal_on["out"];
                    cond_stored["write_en"] = cond_computed ? signal_on["out"];

                    // Compute the body
                    body["go"] = body_go ? signal_on["out"];

                    // Reset the FSM when the body is done.
                    fsm["in"] = body_done ? fsm_reset_val["out"];
                    fsm["write_en"] = body_done ? signal_on["out"];

                    // This group is done when cond is false.
                    while_group["done"] = done ? signal_on["out"];
                );
                assignments.push(builder.build_assignment(
                    cond_stored.borrow().get("in"),
                    Rc::clone(&port),
                    cond_computed,
                ));

                while_group
                    .borrow_mut()
                    .assignments
                    .append(&mut assignments);

                // CLEANUP: Reset the FSM state.
                let mut cleanup = build_assignments!(
                    builder;
                    fsm["in"] = done ? fsm_reset_val["out"];
                    fsm["write_en"] = done ? signal_on["out"];
                );
                comp.continuous_assignments.append(&mut cleanup);

                return Ok(Action::Change(ir::Control::enable(while_group)));
            }
        }

        Ok(Action::Continue)
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        comp: &mut ir::Component,
        ctx: &lib::LibrarySignatures,
    ) -> VisResult {
        if let (ir::Control::Enable(tdata), ir::Control::Enable(fdata)) =
            (&*s.tbranch, &*s.fbranch)
        {
            let cond = &s.cond;
            let tru = &tdata.group;
            let fal = &fdata.group;

            // combinational condition
            if let (Some(&ctime), Some(&ttime), Some(&ftime)) = (
                cond.borrow().attributes.get("static"),
                tru.borrow().attributes.get("static"),
                fal.borrow().attributes.get("static"),
            ) {
                let mut builder = ir::Builder::from(comp, ctx, false);
                let mut attrs = HashMap::new();
                attrs.insert(
                    "static".to_string(),
                    ctime + 1 + cmp::max(ttime, ftime),
                );
                let if_group = builder.add_group("static_if", attrs);

                let fsm_size = 32;
                structure!(builder;
                    let fsm = prim std_reg(fsm_size);
                    let one = constant(1, fsm_size);
                    let signal_on = constant(1, 1);
                    let cond_stored = prim std_reg(1);
                    let reset_val = constant(0, fsm_size);

                    let cond_time_const = constant(ctime, fsm_size);
                    let cond_done_time_const = constant(ctime, fsm_size);

                    let true_end_const = constant(ttime + ctime + 1, fsm_size);
                    let false_end_const = constant(ftime + ctime + 1, fsm_size);

                    let incr = prim std_add(fsm_size);
                );

                let max_const = if ttime > ftime {
                    true_end_const.clone()
                } else {
                    false_end_const.clone()
                };

                // The group is done when we count up to the max.
                let done_guard =
                    guard!(fsm["out"]).eq(guard!(max_const["out"]));
                let not_done_guard = !done_guard.clone();

                // Guard for computing the conditional.
                let cond_go = if ctime == 0 {
                    guard!(fsm["out"]).eq(guard!(cond_time_const["out"]))
                } else {
                    guard!(fsm["out"]).lt(guard!(cond_time_const["out"]))
                };

                // Guard for when the conditional value is available on the
                // port.
                let cond_done =
                    guard!(fsm["out"]).eq(guard!(cond_done_time_const["out"]));

                // Guard for branches
                let true_go = guard!(fsm["out"])
                    .gt(guard!(cond_time_const["out"]))
                    & guard!(fsm["out"]).lt(guard!(true_end_const["out"]))
                    & guard!(cond_stored["out"]);

                let false_go = guard!(fsm["out"])
                    .gt(guard!(cond_time_const["out"]))
                    & guard!(fsm["out"]).lt(guard!(false_end_const["out"]))
                    & !guard!(cond_stored["out"]);

                let save_cond = builder.build_assignment(
                    cond_stored.borrow().get("in"),
                    Rc::clone(&s.port),
                    cond_done.clone(),
                );
                let mut assigns = build_assignments!(builder;
                    // Increment fsm every cycle till end
                    incr["left"] = ? fsm["out"];
                    incr["right"] = ? one["out"];
                    fsm["in"] = not_done_guard ? incr["out"];
                    fsm["write_en"] = not_done_guard ? signal_on["out"];

                    // Compute the cond group
                    cond["go"] = cond_go ? signal_on["out"];

                    // Store the value of the conditional
                    cond_stored["write_en"] = cond_done ? signal_on["out"];

                    // Enable one of the branches
                    tru["go"] = true_go ? signal_on["out"];
                    fal["go"] = false_go ? signal_on["out"];

                    // Group is done when we've counted up to max.
                    if_group["done"] = done_guard ? signal_on["out"];
                );
                if_group.borrow_mut().assignments.append(&mut assigns);
                if_group.borrow_mut().assignments.push(save_cond);

                // CLEANUP: Reset FSM to 0 when computation is finished.
                let mut clean_assigns = build_assignments!(builder;
                    fsm["in"] = done_guard ? reset_val["out"];
                    fsm["write_en"] = done_guard ? signal_on["out"];
                );
                comp.continuous_assignments.append(&mut clean_assigns);

                return Ok(Action::Change(ir::Control::enable(if_group)));
            }
        }

        Ok(Action::Continue)
    }

    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        comp: &mut ir::Component,
        ctx: &lib::LibrarySignatures,
    ) -> VisResult {
        let maybe_max_time = accumulate_static_time(&s.stmts, cmp::max);

        // Early return if this group is not compilable.
        if let Some(max_time) = maybe_max_time {
            let mut builder = ir::Builder::from(comp, ctx, false);

            let mut attrs = HashMap::new();
            attrs.insert("static".to_string(), max_time);

            let par_group = builder.add_group("static_par", attrs);

            // XXX(rachit): Calculate the precise number of states required.
            let fsm_size = 32;
            structure!(builder;
                let fsm = prim std_reg(fsm_size);
                let signal_const = constant(1, 1);
                let incr = prim std_add(fsm_size);
                let one = constant(1, fsm_size);
                let last = constant(max_time, fsm_size);
            );
            let done_guard = guard!(fsm["out"]).eq(guard!(last["out"]));
            let not_done_guard = !done_guard.clone();

            let mut assigns = build_assignments!(builder;
                incr["left"] = ? one["out"];
                incr["right"] = ? fsm["out"];
                fsm["in"] = not_done_guard ? incr["out"];
                fsm["write_en"] = not_done_guard ? signal_const["out"];
                par_group["done"] = done_guard ? signal_const["out"];
            );
            par_group.borrow_mut().assignments.append(&mut assigns);
            for con in s.stmts.iter() {
                if let ir::Control::Enable(data) = con {
                    let group = &data.group;
                    let static_time: u64 = group.borrow().attributes["static"];

                    // group[go] = fsm.out <= static_time ? 1;
                    structure!(builder;
                        let state_const = constant(static_time, fsm_size);
                    );
                    let go_guard =
                        guard!(fsm["out"]).lt(guard!(state_const["out"]));

                    let mut assigns = build_assignments!(builder;
                      group["go"] = go_guard ? signal_const["out"];
                    );
                    par_group.borrow_mut().assignments.append(&mut assigns);
                }
            }

            // CLEANUP: Reset the FSM to initial state.
            structure!(builder;
                let reset_val = constant(0, fsm_size);
            );
            let mut cleanup_assigns = build_assignments!(builder;
                fsm["in"] = done_guard ? reset_val["out"];
                fsm["write_en"] = done_guard ? signal_const["out"];
            );
            comp.continuous_assignments.append(&mut cleanup_assigns);

            Ok(Action::Change(ir::Control::enable(par_group)))
        } else {
            Ok(Action::Continue)
        }
    }

    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        comp: &mut ir::Component,
        ctx: &lib::LibrarySignatures,
    ) -> VisResult {
        // If this sequence only contains groups with the "static" attribute,
        // compile it using a statically timed FSM.
        let total_time = accumulate_static_time(&s.stmts, |acc, x| acc + x);

        // Early return if this group is not compilable.
        if total_time.is_none() {
            return Ok(Action::Continue);
        }

        let mut builder = ir::Builder::from(comp, ctx, false);
        // TODO(rachit): Resize FSM by pre-calculating max value.
        let fsm_size = 32;
        // Create new group for compiling this seq.
        let seq_group = builder.add_group("static_seq", HashMap::new());

        // Add FSM register
        structure!(builder;
            let fsm = prim std_reg(fsm_size);
            let signal_const = constant(1, 1);
        );

        let mut cur_cycle = 0;
        for con in s.stmts.iter() {
            if let ir::Control::Enable(data) = con {
                let group = &data.group;

                // Static time of the group.
                let static_time: u64 = group.borrow().attributes["static"];

                structure!(builder;
                    let start_st = constant(cur_cycle, fsm_size);
                    let end_st = constant(cur_cycle + static_time, fsm_size);
                );

                // group[go] = fsm.out >= start_st & fsm.out < end_st ? 1;
                // NOTE(rachit): Do not generate fsm.out >= 0. Because fsm
                // contains unsigned values, it will always be true and
                // Verilator will generate %Warning-UNSIGNED.
                let go_guard = if static_time == 1 {
                    guard!(fsm["out"]).eq(guard!(start_st["out"]))
                } else if cur_cycle == 0 {
                    guard!(fsm["out"]).le(guard!(end_st["out"]))
                } else {
                    guard!(fsm["out"]).ge(guard!(start_st["out"]))
                        & guard!(fsm["out"]).lt(guard!(end_st["out"]))
                };

                let mut assigns = build_assignments!(builder;
                    group["go"] = go_guard ? signal_const["out"];
                );
                seq_group.borrow_mut().assignments.append(&mut assigns);

                cur_cycle += static_time;
            }
        }

        // Add self incrementing logic for the FSM.
        structure!(builder;
            let incr = prim std_add(fsm_size);
            let one = constant(1, fsm_size);
            let last = constant(cur_cycle, fsm_size);
            let reset_val = constant(0, fsm_size);
        );
        let done_guard = guard!(fsm["out"]).eq(guard!(last["out"]));
        let not_done_guard = !done_guard.clone();

        let mut incr_assigns = build_assignments!(builder;
            incr["left"] = ? one["out"];
            incr["right"] = ? fsm["out"];
            fsm["in"] = not_done_guard ? incr["out"];
            fsm["write_en"] = not_done_guard ? signal_const["out"];
            seq_group["done"] = done_guard ? signal_const["out"];
        );
        seq_group.borrow_mut().assignments.append(&mut incr_assigns);

        // CLEANUP: Reset the fsm to initial state once it's done.
        let mut cleanup_assigns = build_assignments!(builder;
            fsm["in"] = done_guard ? reset_val["out"];
            fsm["write_en"] = done_guard ? signal_const["out"];
        );
        comp.continuous_assignments.append(&mut cleanup_assigns);

        // Add static attribute to this group.
        seq_group
            .borrow_mut()
            .attributes
            .insert("static".to_string(), cur_cycle);

        // Replace the control with the seq group.
        Ok(Action::Change(ir::Control::enable(seq_group)))
    }
}
