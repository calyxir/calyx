use crate::errors::Extract;
use crate::lang::ast::{Control, Enable};
use crate::lang::{
    ast, component::Component, context::Context, structure::StructureGraph,
    structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use crate::{add_wires, port, structure};
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
                st.groups
                    .get(&Some(group.clone()))
                    .and_then(|(attrs, _)| attrs.get("static"))
                    .ok_or_else(|| ())
            } else {
                Err(())
            }
        })
        .collect();

    timing.ok().map(|ts| ts.into_iter().fold(0, acc))
}

impl Visitor for StaticTiming {
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
                st.to_guard(port!(st; fsm."out")).eq(st.to_guard(last));
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
                    let go_guard = st
                        .to_guard(port!(st; fsm."out"))
                        .le(st.to_guard(state_const));
                    st.insert_edge(
                        signal_const.clone(),
                        port!(st; group["go"]),
                        Some(par_group.clone()),
                        Some(go_guard),
                    )?;
                }
            }

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

                // group[go] = fsm.out == cur_cyle ? 1;
                structure!(st, &ctx,
                    let state_const = constant(cur_cycle, fsm_size);
                );
                let go_guard = st
                    .to_guard(port!(st; fsm."out"))
                    .eq(st.to_guard(state_const));
                st.insert_edge(
                    signal_const.clone(),
                    port!(st; group["go"]),
                    Some(seq_group.clone()),
                    Some(go_guard),
                )?;

                // Update `cur_cycle` to the cycle on which this group
                // finishes executing.
                let static_time: u64 = *st
                    .groups
                    .get(&Some(group_name.clone()))
                    .expect("Group missing from structure")
                    .0
                    .get("static")
                    .expect(
                        "Impossible: Group doesn't have \"static\" attribute",
                    );
                cur_cycle += static_time;
            }
        }

        // Add self incrementing logic for the FSM.
        structure!(st, &ctx,
            let incr = prim std_add(fsm_size);
            let one = constant(1, fsm_size);
            let last = constant(cur_cycle, fsm_size);
        );
        let done_guard =
            st.to_guard(port!(st; fsm."out")).eq(st.to_guard(last));
        let not_done_guard = !done_guard.clone();

        add_wires!(st, Some(seq_group.clone()),
            incr["left"] = (one);
            incr["right"] = (fsm["out"]);
            fsm["in"] = not_done_guard ? (incr["out"]);
            fsm["write_en"] = not_done_guard ? (signal_const.clone());
            seq_group_node["done"] = done_guard ? (signal_const);
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
