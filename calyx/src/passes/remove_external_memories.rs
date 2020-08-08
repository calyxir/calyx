use crate::lang::ast;
use crate::lang::component::Component;
use crate::lang::context::Context;
use crate::lang::structure::{Node, NodeData};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use ast::Cell;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

#[derive(Default)]
pub struct RemoveExternalMemories;

impl Named for RemoveExternalMemories {
    fn name() -> &'static str {
        "remove-external-memories"
    }

    fn description() -> &'static str {
        "replace external memory primitives with internal memory primitives"
    }
}

impl Visitor for RemoveExternalMemories {
    fn start(&mut self, comp: &mut Component, c: &Context) -> VisResult {
        let st = &mut comp.structure;

        let changeable: HashMap<&'static str, &'static str> = vec![
            ("std_mem_d1_ext", "std_mem_d1"),
            ("std_mem_d2_ext", "std_mem_d2"),
            ("std_mem_d3_ext", "std_mem_d3"),
        ]
        .into_iter()
        .collect();

        // gather components to change
        let to_change = st
            .component_iterator()
            .filter_map(|(idx, node)| {
                if let NodeData::Cell(Cell::Prim { data }) = &node.data {
                    if changeable.contains_key(data.instance.name.as_ref()) {
                        Some(idx)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<NodeIndex>>();

        for idx in to_change {
            let node: &mut Node = st.get_node_mut(idx);
            if let NodeData::Cell(Cell::Prim { ref mut data }) = &mut node.data
            {
                // new prim name
                let prim_name: ast::Id =
                    changeable[data.instance.name.as_ref()].into();

                // new prim signature
                let resolved_sig = c
                    .library_context
                    .resolve(&prim_name, &data.instance.params)
                    .expect("Primitive wasn't able to be resolved.");

                // change node type
                data.instance.name = prim_name;
                node.signature = resolved_sig;
            } else {
                unreachable!("Already filtered for NodeData::Cell")
            }
        }

        Ok(Action::Stop)
    }
}
