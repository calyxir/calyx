use crate::lang::ast::{Portdef, Cell};
use crate::lang::component::Component;
use crate::lang::context::Context;
use crate::lang::structure::{DataDirection, Node, NodeData};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use petgraph::graph::NodeIndex;

#[derive(Default)]
pub struct Externalize;

impl Named for Externalize {
    fn name() -> &'static str {
        "externalize"
    }

    fn description() -> &'static str {
        "externalize the interfaces of _ext memories"
    }
}

impl Visitor for Externalize {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        let st = &mut comp.structure;

        let indicies = st
            .component_iterator()
            .filter_map(|(idx, node)| {
                if let NodeData::Cell(Cell::Prim { data }) = &node.data {
                    if data.instance.name.as_ref().starts_with("std_mem") &&
                        data.instance.name.as_ref().ends_with("ext") {
                        Some((idx, node.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<(NodeIndex, Node)>>();

        for (idx, node) in indicies {
            for portdef in &node.signature.inputs {
                let portname = format!("{}_{}", node.name.as_ref(), portdef.name.as_ref());
                let new_portdef = Portdef {
                    name: portname.into(),
                    width: portdef.width
                };
                st.insert_output_port(&new_portdef);
                for edidx in st
                    .edge_idx()
                    .with_node(idx)
                    .with_port(portdef.name.to_string())
                    .with_direction(DataDirection::Write)
                    .detach()
                {
                    let edge = st.get_edge(edidx).clone();
                    let src_port = edge.src.port_name().clone();
                    let (src_node, _) = st.endpoints(edidx);

                    st.insert_edge(
                        (src_node, src_port),
                        (st.get_this_idx(), new_portdef.name.clone()),
                        edge.group,
                        edge.guard,
                    )?;

                    st.remove_edge(edidx);
                }
            }

            for portdef in &node.signature.outputs {
                let portname = format!("{}_{}", node.name.as_ref(), portdef.name.as_ref());
                let new_portdef = Portdef {
                    name: portname.into(),
                    width: portdef.width
                };
                st.insert_input_port(&new_portdef);
                for edidx in st
                    .edge_idx()
                    .with_node(idx)
                    .with_port(portdef.name.to_string())
                    .with_direction(DataDirection::Read)
                    .detach()
                {
                    let edge = st.get_edge(edidx).clone();
                    let dest_port = edge.dest.port_name().clone();
                    let (_, dest_node) = st.endpoints(edidx);

                    st.insert_edge(
                        (st.get_this_idx(), new_portdef.name.clone()),
                        (dest_node, dest_port),
                        edge.group,
                        edge.guard,
                    )?;

                    st.remove_edge(edidx);
                }
            }

            st.remove_node(idx);
        }

        // Stop traversal, we don't need to traverse over control ast
        Ok(Action::Stop)
    }
}
