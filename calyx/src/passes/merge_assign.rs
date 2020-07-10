//use crate::errors::{Error, Extract};
use crate::lang::ast::GuardExpr;
use crate::lang::{
    component::Component, context::Context, structure::DataDirection,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use itertools::Itertools;

#[derive(Default)]
pub struct MergeAssign {}

impl Named for MergeAssign {
    fn name() -> &'static str {
        "compile-control"
    }

    fn description() -> &'static str {
        "Compile away all control language constructs into structure"
    }
}

impl Visitor for MergeAssign {
    fn start(&mut self, comp: &mut Component, _ctx: &Context) -> VisResult {
        //XXX(rachit): This code ignores groups.
        let structure = &comp.structure;
        let mut merged_edges = Vec::new();

        for (idx, node) in structure.component_iterator() {
            for portdef in node.signature.inputs.iter() {
                // For each (node, port) being written into, collect all
                // HashMap<rhs, Vec<(width, guards)>> values.
                let combined_guard = comp
                    .structure
                    .edge_idx()
                    .with_node(idx)
                    .with_port(portdef.name.to_string())
                    .with_direction(DataDirection::Write)
                    .map(|idx| {
                        let ed = structure.get_edge(idx);
                        let (src_node, _) = structure.endpoints(idx);
                        ((src_node, ed.src.port_name().clone()), &ed.guard)
                    })
                    .into_group_map()
                    .into_iter()
                    .map(|(src, guards)| {
                        (
                            src,
                            GuardExpr::or_vec(
                                guards
                                    .into_iter()
                                    .filter_map(|g| g.clone())
                                    .collect(),
                            ),
                        )
                    })
                    .collect::<Vec<_>>();

                let dest = (idx, &portdef.name);
                merged_edges.push((dest, combined_guard));
            }
        }

        for ((dest_idx, dest_port), edges) in merged_edges {
            for (src, guard) in edges {
                comp.structure.insert_edge(
                    src,
                    (dest_idx, dest_port.clone()),
                    None,
                    Some(guard),
                )?;
            }
        }

        Ok(Action::Stop)
    }
}
