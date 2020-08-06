use crate::lang::component::Component;
use crate::lang::{ast, context::Context, structure, structure_builder};
use crate::{
    add_wires,
    passes::visitor::{Action, Named, VisResult, Visitor},
    port,
};
use ast::Cell;
use petgraph::graph::NodeIndex;
use std::collections::HashSet;
use structure::{DataDirection, NodeData};
use structure_builder::ASTBuilder;

#[derive(Default)]
pub struct DspRegInsertion;

impl Named for DspRegInsertion {
    fn name() -> &'static str {
        "dsp-reg-insertion"
    }

    fn description() -> &'static str {
        "inserts registers around DSP primitives to meet timing"
    }
}

lazy_static::lazy_static! {
    static ref DSP_PRIMITIVES: HashSet<&'static str> = vec![
        "std_mult"
    ].into_iter().collect();
}

impl Visitor for DspRegInsertion {
    fn start(&mut self, comp: &mut Component, ctx: &Context) -> VisResult {
        let st = &mut comp.structure;

        let to_wrap = st
            .component_iterator()
            .filter_map(|(idx, node)| {
                if let NodeData::Cell(Cell::Prim { data }) = &node.data {
                    if DSP_PRIMITIVES.contains(data.instance.name.as_ref()) {
                        Some(idx)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<NodeIndex>>();

        for idx in to_wrap {
            for edge_idx in st
                .edge_idx()
                .with_node(idx)
                .with_direction(DataDirection::Write)
                .detach()
            {
                let edge = st.get_edge(edge_idx).clone();
                let (src, _) = st.endpoints(edge_idx);
                let reg = st.new_primitive(
                    &ctx,
                    "dsp_reg",
                    "std_reg",
                    &[edge.width],
                )?;

                add_wires!(st, edge.group,
                           reg["in"] = (src[edge.src.port_name()]);
                           idx[edge.dest.port_name()] = (reg["out"]);
                );
            }

            for edge_idx in st
                .edge_idx()
                .with_node(idx)
                .with_direction(DataDirection::Read)
                .detach()
            {
                let edge = st.get_edge(edge_idx).clone();
                let (_, dest) = st.endpoints(edge_idx);
                let reg = st.new_primitive(
                    &ctx,
                    "dsp_reg",
                    "std_reg",
                    &[edge.width],
                )?;

                add_wires!(st, edge.group,
                           reg["in"] = (idx[edge.src.port_name()]);
                           dest[edge.dest.port_name()] = (reg["out"]);
                );
            }
        }

        Ok(Action::Continue)
    }
}
