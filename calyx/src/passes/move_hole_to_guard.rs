use crate::lang::component::Component;
use crate::lang::{
    context::Context, structure, structure_builder, structure_iter::NodeType,
};
use crate::{
    add_wires, guard,
    passes::visitor::{Action, Named, VisResult, Visitor},
    port, structure,
};
use structure::DataDirection;
use structure_builder::ASTBuilder;

#[derive(Default)]
pub struct MoveHoleToGuard;

impl Named for MoveHoleToGuard {
    fn name() -> &'static str {
        "move-hole-to-guard"
    }

    fn description() -> &'static str {
        "move reads from a hole into the guard"
    }
}

impl Visitor for MoveHoleToGuard {
    fn start(&mut self, comp: &mut Component, _ctx: &Context) -> VisResult {
        let st = &mut comp.structure;

        for edge_idx in st
            .edge_idx()
            .with_direction(DataDirection::Read)
            .with_node_type(NodeType::Hole)
            .detach()
        {
            let (src, dest) = st.endpoints(edge_idx);
            let edge = st.get_edge(edge_idx).clone();
            let guard = match edge.guard {
                Some(g) => guard!(st; src[edge.src.port_name()]) & g,
                None => guard!(st; src[edge.src.port_name()]),
            };

            structure!(
                st, &ctx,
                let one = constant(1, 1);
            );

            add_wires!(
                st, edge.group,
                dest[edge.dest.port_name()] = guard ? (one);
            );

            st.remove_edge(edge_idx);
        }

        Ok(Action::Stop)
    }
}
