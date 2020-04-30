use crate::errors::Error;
use crate::lang::ast::Control;
use crate::lang::structure::NodeData;
use crate::lang::{component::Component, context::Context};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

#[derive(Default)]
pub struct GroupPassthrough;

impl Named for GroupPassthrough {
    fn name() -> &'static str {
        "group-passthrough"
    }

    fn description() -> &'static str {
        "erase groups by replacing `a -> g @ valid -> b` with `a -> b`"
    }
}

impl Visitor for GroupPassthrough {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        let mut new_edges = vec![];
        for (idx, node) in comp.structure.nodes() {
            if let NodeData::Group(..) = node.data {
                for (src, e1) in comp.structure.incoming_to_port(idx, "clk") {
                    for (dest, e2) in
                        comp.structure.outgoing_from_port(idx, "clk")
                    {
                        let src_idx = match src.data {
                            NodeData::Port => comp.structure.get_this(),
                            _ => comp.structure.get_idx(&src.name)?,
                        };
                        let dest_idx = match dest.data {
                            NodeData::Port => comp.structure.get_this(),
                            _ => comp.structure.get_idx(&dest.name)?,
                        };
                        new_edges.push((
                            src_idx,
                            e1.src.clone(),
                            dest_idx,
                            e2.dest.clone(),
                        ))
                    }
                }
            }
        }

        println!("{:?}", new_edges);

        for (si, sp, di, dp) in new_edges {
            comp.structure.insert_edge(si, sp, di, dp)?;
        }

        Ok(Action::Stop)
    }
}
