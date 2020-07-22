use crate::errors::Extract;
use crate::lang::ast::{Control, Enable};
use crate::lang::{
    ast, component::Component, context::Context, structure::StructureGraph,
    structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use crate::{add_wires, port, structure};
//use itertools::Itertools;
use std::collections::HashMap;
//use petgraph::graph::NodeIndex;

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

impl Visitor for StaticTiming {
    fn finish_seq(
        &mut self,
        s: &ast::Seq,
        comp: &mut Component,
        ctx: &Context,
    ) -> VisResult {
        // If this sequence only contains groups with the "static" attribute,
        // compile it using a statically timed FSM.
        let is_compilable = s.stmts.iter().all(|con| match con {
            Control::Enable {
                data: Enable { comp: group },
            } => comp
                .structure
                .groups
                .get(&Some(group.clone()))
                .map_or(false, |(attrs, _)| attrs.contains_key("static")),
            _ => false,
        });

        // Early return if this group is not compilable.
        if !is_compilable {
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
            let fsm_reg = prim std_reg(fsm_size);
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
                    .to_guard(port!(st; fsm_reg."out"))
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
            st.to_guard(port!(st; fsm_reg."out")).eq(st.to_guard(last));
        let not_done_guard = !done_guard.clone();

        add_wires!(st, seq_group,
            incr["left"] = (one);
            incr["right"] = (fsm_reg["out"]);
            fsm_reg["in"] = not_done_guard ? (incr["out"]);
            fsm_reg["write_en"] = not_done_guard ? (signal_const.clone());
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
