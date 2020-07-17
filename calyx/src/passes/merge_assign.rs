//use crate::errors::{Error, Extract};
use crate::lang::ast::{GuardExpr, Id};
use crate::lang::{
    component::Component, context::Context, structure::DataDirection,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use itertools::Itertools;
use petgraph::graph::NodeIndex;

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

/// Merge assignments of the form:
/// ```
/// n.p = g1 ? x;
/// n.p = g2 ? x;
/// n.p = g3 ? y;
/// ```
/// into:
/// ```
/// n.p = g1 | g2 ? x;
/// n.p = g3 ? y;
/// ```
impl Visitor for MergeAssign {
    fn start(&mut self, comp: &mut Component, _ctx: &Context) -> VisResult {
        assert!(
            comp.structure.groups.len() == 1,
            "MergeAssign: The component has more than one group."
        );

        type Assignments = Vec<(
            /* (dest, port) being written to */
            (NodeIndex, Id),
            /* (src, port) and their guards */
            Vec<((NodeIndex, Id), GuardExpr)>,
        )>;

        let mut merged_edges: Assignments = Vec::new();

        let structure = &comp.structure;
        for (idx, node) in structure.component_iterator() {
            for portdef in node.signature.inputs.iter() {
                let iterator = structure
                    .edge_idx()
                    .with_node(idx)
                    .with_port(portdef.name.to_string())
                    .with_direction(DataDirection::Write)
                    .detach();
                // For each (node, port) being written into, collect all
                // HashMap<rhs, Vec<(width, guards)>> values and remove
                // the edges.
                let combined_guard = iterator
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

                let dest = (idx, portdef.name.clone());
                merged_edges.push((dest, combined_guard));
            }
        }

        for e_idx in structure.edge_idx().detach() {
            comp.structure.remove_edge(e_idx);
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
