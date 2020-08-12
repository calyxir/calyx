use crate::lang::{
    ast, component::Component, context::Context, structure_builder::ASTBuilder,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use crate::{add_wires, port, structure};
use ast::Control;
use std::collections::HashMap;

#[derive(Default)]
pub struct CompileEmpty {}

impl CompileEmpty {
    const EMPTY_GROUP: &'static str = "_empty";
}

impl Named for CompileEmpty {
    fn name() -> &'static str {
        "compile-empty"
    }

    fn description() -> &'static str {
        "Rewrites empty control to invocation to empty group"
    }
}

impl Visitor for CompileEmpty {
    fn finish_empty(
        &mut self,
        _s: &ast::Empty,
        comp: &mut Component,
        _x: &Context,
    ) -> VisResult {
        let st = &mut comp.structure;
        // Create a group that always outputs done if it doesn't exist.
        let empty_group: ast::Id = CompileEmpty::EMPTY_GROUP.into();
        let mut attrs = HashMap::new();
        attrs.insert("static".to_string(), 0);
        // Try to get the empty group.
        if st.get_node_by_name(&empty_group).is_err() {
            let empty_group_node = st.insert_group(&empty_group, attrs)?;

            structure!(st, &ctx,
                let signal_on = constant(1, 1);
            );

            add_wires!(
                st, Some(empty_group.clone()),
                empty_group_node["done"] = (signal_on);
            );
        }
        Ok(Action::Change(Control::enable(empty_group)))
    }
}
