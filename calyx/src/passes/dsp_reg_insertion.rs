use crate::lang::component::Component;
use crate::lang::{ast, context::Context, structure};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use ast::Cell;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use structure::{Node, NodeData};

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
    static ref DSP_PRIMITIVES: HashMap<&'static str, &'static str> = vec![
        ("std_mult", "std_mult_pipe")
    ].into_iter().collect();
}

impl Visitor for DspRegInsertion {
    fn start(&mut self, comp: &mut Component, ctx: &Context) -> VisResult {
        let st = &mut comp.structure;

        let to_wrap = st
            .component_iterator()
            .filter_map(|(idx, node)| {
                if let NodeData::Cell(Cell::Prim { data }) = &node.data {
                    if DSP_PRIMITIVES.contains_key(data.instance.name.as_ref())
                    {
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
            let node: &mut Node = st.get_node_mut(idx);
            if let NodeData::Cell(Cell::Prim { ref mut data }) = &mut node.data
            {
                // new prim name
                let prim_name: ast::Id =
                    DSP_PRIMITIVES[data.instance.name.as_ref()].into();

                // new prim signature
                let resolved_sig = ctx
                    .library_context
                    .resolve(&prim_name, &data.instance.params)
                    .expect("Primitive wasn't able to be resolved.");

                // change node type
                data.instance.name = prim_name;
                node.signature = resolved_sig;
            } else {
                unreachable!("Already filtered for NodeData::Cell")
            }

            // let mut group = None;

            // // register write into idx wires
            // for edge_idx in st
            //     .edge_idx()
            //     .with_node(idx)
            //     .with_direction(DataDirection::Write)
            //     .detach()
            // {
            //     let edge = st.get_edge(edge_idx).clone();

            //     match group {
            //         Some(_) if group == edge.group => (),
            //         Some(_) => panic!("Mulitple groups"),
            //         None => group = edge.group.clone(),
            //     }

            //     let (src, _) = st.endpoints(edge_idx);

            //     structure!(
            //         st, &ctx,
            //         let dsp_inp = prim std_reg_no_done(edge.width);
            //     );

            //     add_wires!(
            //         st, edge.group,
            //         dsp_inp["in"] = (src[edge.src.port_name()]);
            //         idx[edge.dest.port_name()] = (dsp_inp["out"]);
            //     );

            //     st.remove_edge(edge_idx);
            // }

            // // register read from idx wires
            // for edge_idx in st
            //     .edge_idx()
            //     .with_node(idx)
            //     .with_direction(DataDirection::Read)
            //     .detach()
            // {
            //     let edge = st.get_edge(edge_idx).clone();

            //     match group {
            //         Some(_) if group == edge.group => (),
            //         Some(_) => panic!("Mulitple groups"),
            //         None => group = edge.group.clone(),
            //     }

            //     let (_, dest) = st.endpoints(edge_idx);

            //     structure!(
            //         st, &ctx,
            //         let dsp_outp = prim std_reg_no_done(edge.width);
            //     );

            //     add_wires!(
            //         st, edge.group,
            //         dsp_outp["in"] = (idx[edge.src.port_name()]);
            //         dest[edge.dest.port_name()] = (dsp_outp["out"]);
            //     );

            //     st.remove_edge(edge_idx);
            // }

            // // increase static time of group
            // st.groups
            //     .get_mut(&group)
            //     .expect("Missing group")
            //     .0
            //     .entry("static".into())
            //     .and_modify(|time| *time += 3);
        }

        Ok(Action::Continue)
    }
}
