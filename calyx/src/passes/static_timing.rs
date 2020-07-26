use crate::errors::Extract;
use crate::lang::ast::{Control, Enable};
use crate::lang::{
    ast, component::Component, context::Context, structure::StructureGraph,
    structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use crate::{guard, add_wires, port, structure};
use std::cmp;
use std::collections::HashMap;

#[derive(Default)]
pub struct StaticTiming {}

impl Named for StaticTiming {
    fn name() -> &'static str {
        "static-timing"
    }

    fn description() -> &'static str {
        "Opportunisitcally compile timed groups and generate timing information when possible."
    }
}

/// Function to iterate over a vector of control statements and collect
/// the "static" attribute using the `acc` function.
/// Returns None if any of of the Control statements is a compound statement.
fn accumulate_static_time<F>(
    st: &StructureGraph,
    stmts: &[Control],
    acc: F,
) -> Option<u64>
where
    F: FnMut(u64, &u64) -> u64,
{
    let timing: Result<Vec<&u64>, ()> = stmts
        .iter()
        .map(|con| {
            if let Control::Enable {
                data: Enable { comp: group },
            } = con
            {
                st.groups[&Some(group.clone())]
                    .0
                    .get("static")
                    .ok_or_else(|| ())
            } else {
                Err(())
            }
        })
        .collect();

    timing.ok().map(|ts| ts.into_iter().fold(0, acc))
}

impl Visitor for StaticTiming {
    /*fn finish_if(
        &mut self,
        s: &ast::If,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        let st = &mut comp.structure;

        if let (
            Control::Enable { data: tdata },
            Control::Enable { data: fdata },
        ) = (&*s.tbranch, &*s.fbranch)
        {
            let maybe_cond_time =
                st.groups[&Some(s.cond.clone())].0.get("static");
            let maybe_true_time =
                st.groups[&Some(tdata.comp.clone())].0.get("static");
            let maybe_false_time =
                st.groups[&Some(fdata.comp.clone())].0.get("static");

            if let (Some(&ctime), Some(&ttime), Some(&ftime)) =
                (maybe_cond_time, maybe_true_time, maybe_false_time)
            {
                let if_group: ast::Id = st.namegen.gen_name("if").into();
                let if_group_node = st.insert_group(&if_group, HashMap::new());

                let fsm_size = 32;
                structure!(st, &ctx,
                    let fsm = prim std_reg(fsm_size);
                    let one = constant(1, fsm_size);
                    let signal_on = constant(1, 1);
                    let cond_stored = prim std_reg(1);

                    let cond_time_const = constant(ctime, fsm_size);
                    let true_time_const = constant(ttime, fsm_size);
                    let false_time_const = constant(ftime, fsm_size);

                    let incr = prim std_add(fsm_size);
                );

                let max_const = if ttime > ftime {
                    true_time_const
                } else {
                    false_time_const
                };

                // The group is done when we count up to the max.
                let done_guard = st
                    .to_guard(port!(st; fsm["out"]))
                    .eq(st.to_guard(max_const));
                let not_done_guard = !done_guard;

                add_wires!(st,
                    // Increment fsm every cycle till end.
                    add["left"] = (fsm["out"]);
                    add["right"] = (one);
                    fsm["in"] = not_done_guard ? (incr["out"]);

                    // Compute the cond group
                );
            }
        }

        Ok(Action::Continue)
    }*/

    fn finish_par(
        &mut self,
        s: &ast::Par,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        let maybe_max_time =
            accumulate_static_time(&comp.structure, &s.stmts, |acc, x| {
                cmp::max(acc, *x)
            });

        // Early return if this group is not compilable.
        if let Some(max_time) = maybe_max_time {
            let st = &mut comp.structure;

            let mut attrs = HashMap::new();
            attrs.insert("static".to_string(), max_time);

            let par_group: ast::Id = st.namegen.gen_name("static_par").into();
            let par_group_node = st.insert_group(&par_group, attrs)?;

            // XXX(rachit): Calculate the precise number of states required.
            let fsm_size = 32;
            structure!(st, &ctx,
                let fsm = prim std_reg(fsm_size);
                let signal_const = constant(1, 1);
                let incr = prim std_add(fsm_size);
                let one = constant(1, fsm_size);
                let last = constant(max_time, fsm_size);
            );
            let done_guard =
                guard!(st; fsm["out"]).eq(st.to_guard(last));
            let not_done_guard = !done_guard.clone();

            add_wires!(st, Some(par_group.clone()),
                incr["left"] = (one);
                incr["right"] = (fsm["out"]);
                fsm["in"] = not_done_guard ? (incr["out"]);
                fsm["write_en"] = not_done_guard ? (signal_const.clone());
                par_group_node["done"] = done_guard ? (signal_const.clone());
            );
            for con in s.stmts.iter() {
                if let Control::Enable {
                    data: Enable { comp: group_name },
                } = con
                {
                    let group = st
                        .get_node_by_name(&group_name)
                        .extract(group_name.clone())?;

                    let static_time: u64 = *st
                    .groups
                    .get(&Some(group_name.clone()))
                    .expect("Group missing from structure")
                    .0
                    .get("static")
                    .expect(
                        "Impossible: Group doesn't have \"static\" attribute",
                    );

                    // group[go] = fsm.out <= static_time ? 1;
                    structure!(st, &ctx,
                        let state_const = constant(static_time, fsm_size);
                    );
                    let go_guard = guard!(st; fsm["out"])
                        .le(st.to_guard(state_const));
                    add_wires!(st, Some(par_group.clone()),
                      group["go"] = go_guard ? (signal_const.clone());
                    );
                }
            }

            // CLEANUP: Reset the FSM to initial state.
            structure!(st, &ctx,
                let reset_val = constant(0, fsm_size);
            );
            add_wires!(st, None,
                fsm["in"] = done_guard ? (reset_val);
                fsm["write_en"] = done_guard ? (signal_const);
            );

            Ok(Action::Change(Control::enable(par_group)))
        } else {
            Ok(Action::Continue)
        }
    }

    fn finish_seq(
        &mut self,
        s: &ast::Seq,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        // If this sequence only contains groups with the "static" attribute,
        // compile it using a statically timed FSM.
        let total_time =
            accumulate_static_time(&comp.structure, &s.stmts, |acc, x| acc + x);

        // Early return if this group is not compilable.
        if total_time.is_none() {
            return Ok(Action::Continue);
        }

        let st = &mut comp.structure;
        // TODO(rachit): Resize FSM by pre-calculating max value.
        let fsm_size = 32;
        // Create new group for compiling this seq.
        let seq_group: ast::Id = st.namegen.gen_name("static_seq").into();
        let seq_group_node = st.insert_group(&seq_group, HashMap::new())?;

        // Add FSM register
        structure!(st, &ctx,
            let fsm = prim std_reg(fsm_size);
            let signal_const = constant(1, 1);
        );

        let mut cur_cycle = 0;
        for con in s.stmts.iter() {
            if let Control::Enable {
                data: Enable { comp: group_name },
            } = con
            {
                let group = st
                    .get_node_by_name(&group_name)
                    .extract(group_name.clone())?;

                // Static time of the group.
                let static_time: u64 = *st
                    .groups
                    .get(&Some(group_name.clone()))
                    .expect("Group missing from structure")
                    .0
                    .get("static")
                    .expect(
                        "Impossible: Group doesn't have \"static\" attribute",
                    );

                structure!(st, &ctx,
                    let start_st = constant(cur_cycle, fsm_size);
                    let end_st = constant(cur_cycle + static_time, fsm_size);
                );

                // group[go] = fsm.out >= start_st & fsm.out < end_st ? 1;
                // NOTE(rachit): Do not generate fsm.out >= 0. Because fsm
                // contains unsigned values, it will always be true and
                // Verilator will generate %Warning-UNSIGNED.
                let go_guard = if static_time == 1 {
                    guard!(st; fsm["out"]).eq(st.to_guard(start_st))
                } else if cur_cycle == 0 {
                    guard!(st; fsm["out"]).le(st.to_guard(end_st))
                } else {
                    guard!(st; fsm["out"]).ge(st.to_guard(start_st))
                        & guard!(st; fsm["out"])
                            .lt(st.to_guard(end_st))
                };

                add_wires!(st, Some(seq_group.clone()),
                    group["go"] = go_guard ? (signal_const.clone());
                );

                cur_cycle += static_time;
            }
        }

        // Add self incrementing logic for the FSM.
        structure!(st, &ctx,
            let incr = prim std_add(fsm_size);
            let one = constant(1, fsm_size);
            let last = constant(cur_cycle, fsm_size);
            let reset_val = constant(0, fsm_size);
        );
        let done_guard =
            guard!(st; fsm["out"]).eq(st.to_guard(last));
        let not_done_guard = !done_guard.clone();

        add_wires!(st, Some(seq_group.clone()),
            incr["left"] = (one);
            incr["right"] = (fsm["out"]);
            fsm["in"] = not_done_guard ? (incr["out"]);
            fsm["write_en"] = not_done_guard ? (signal_const.clone());
            seq_group_node["done"] = done_guard ? (signal_const.clone());
        );

        // CLEANUP: Reset the fsm to initial state once it's done.
        add_wires!(st, None,
            fsm["in"] = done_guard ? (reset_val);
            fsm["write_en"] = done_guard ? (signal_const);
        );

        // Add static attribute to this group.
        st.groups
            .get_mut(&Some(seq_group.clone()))
            .expect("Missing group")
            .0
            .insert("static".to_string(), cur_cycle);

        // Replace the control with the seq group.
        Ok(Action::Change(Control::enable(seq_group)))
    }
}
