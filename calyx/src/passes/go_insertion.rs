use crate::lang::component::Component;
use crate::lang::{ast, context::Context, structure_ext::ConnectionIteration};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

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
        let iteration = ConnectionIteration::default();
        for edge_data in st.edge_iterator_mut(iteration)? {
            if let Some(group_name) = &edge_data.group {
                let group_go = ast::Port::Hole {
                    group: group_name.clone(),
                    name: "go".into(),
                };

                let go_guard = ast::GuardExpr::Atom(ast::Atom::Port(group_go));
                edge_data.guard.guard.push(go_guard)
            }
        }

        // This pass doesn't modify any control.
        Ok(Action::Stop)
    }
}
