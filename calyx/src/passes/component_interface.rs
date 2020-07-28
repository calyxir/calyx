use crate::errors::Error;
use crate::lang::{
    ast, component::Component, context::Context, structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use crate::{add_wires, guard, port, structure};
use ast::Control;

#[derive(Default)]
pub struct ComponentInterface;

impl Named for ComponentInterface {
    fn name() -> &'static str {
        "component-interface-inserter"
    }

    fn description() -> &'static str {
        "create a go/done interface for components and wire up a single enable to this interface"
    }
}

impl Visitor for ComponentInterface {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        // add go/done signals XXX(sam) this is temporary until we have a more structured sol
        comp.add_input(("go", 1))?;
        comp.add_output(("done", 1))?;

        let st = &mut comp.structure;
        let this = st.get_node_by_name(&"this".into()).unwrap();

        // Guard all connections that are not inside a group.
        let go_guard = guard!(st; this["go"]);
        let edges = st.groups.get(&None).expect("No default group").1.clone();
        for e_idx in edges {
            let mut edge_ref = st.get_edge_mut(e_idx);
            let new_guard = match &edge_ref.guard {
                Some(g) => g.clone() & go_guard.clone(),
                None => go_guard.clone(),
            };
            edge_ref.guard = Some(new_guard);
        }

        if let Control::Enable { data } = &comp.control {
            let group = st.get_node_by_name(&data.comp)?;

            structure!(st, &ctx,
                let num = constant(1, 1);
            );
            let group_done = guard!(st; group["done"]);
            add_wires!(st, None,
                group["go"] = (this["go"]);
                this["done"] = group_done ? (num);
            );

            // this pass doesn't modify any control, so we can return immediately
            Ok(Action::Stop)
        } else if let Control::Empty { .. } = &comp.control {
            Ok(Action::Stop)
        } else {
            Err(Error::MalformedControl(
                "ComponentInterface: Structure has more than one group"
                    .to_string(),
            ))
        }
    }
}
