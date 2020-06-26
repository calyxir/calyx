use crate::errors::{Error, Extract};
use crate::lang::{
    ast, component::Component, context::Context, structure,
    structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use crate::port;
use ast::{Control, GuardExpr};

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

        if let Control::Enable { data } = &comp.control {
            let group =
                st.get_node_by_name(&data.comp).extract(data.comp.clone())?;

            st.insert_edge(
                port!(st; this."go"),
                port!(st; group["go"]),
                None,
                vec![],
            )?;
            let num = st.new_constant(1, 1)?;
            st.insert_edge(
                num,
                port!(st; this."done"),
                None,
                vec![GuardExpr::Atom(st.to_atom(port!(st; group["done"])))],
            )?;

            // this pass doesn't modify any control, so we can return immediately
            Ok(Action::Stop)
        } else {
            Err(Error::MalformedControl("terst".to_string()))
        }
    }
}
