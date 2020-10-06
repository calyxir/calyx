use crate::lang::component::Component;
use crate::lang::{ast, context::Context, structure};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use structure::NodeData;

#[derive(Default)]
pub struct GoInsertion {}

impl Named for GoInsertion {
    fn name() -> &'static str {
        "go-insertion"
    }

    fn description() -> &'static str {
        "removes redudant seq statements"
    }
}

impl Visitor for GoInsertion {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        let st = &mut comp.structure;
        for edge_idx in st.edge_idx().detach() {
            let (_src, dest) = st.endpoints(edge_idx);
            let is_hole = matches!(st.get_node(dest).data, NodeData::Hole(..));
            let edge_data = st.get_edge_mut(edge_idx);
            if !(is_hole && edge_data.dest.port_name() == "done") {
                if let Some(group_name) = &edge_data.group {
                    let group_go = ast::Port::Hole {
                        group: group_name.clone(),
                        name: "go".into(),
                    };

                    let go_guard =
                        ast::GuardExpr::Atom(ast::Atom::Port(group_go));
                    edge_data.guard = Some(match &edge_data.guard {
                        Some(g) => g.clone() & go_guard,
                        None => go_guard,
                    });
                }
            }
        }

        // This pass doesn't modify any control.
        Ok(Action::Stop)
    }
}
