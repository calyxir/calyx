use crate::errors::Error;
use crate::lang::{
    ast, component::Component, context::Context, structure::NodeData,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use ast::Enable;
use std::collections::HashSet;

pub struct WellFormed {
    /// Set of names that components and cells are not allowed to have.
    reserved_names: HashSet<String>,
}

impl Default for WellFormed {
    fn default() -> Self {
        let reserved_names = vec![
            "reg", "wire", "always", "posedge", "negedge", "logic", "tri",
            "input", "output", "if", "generate", "var",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        WellFormed { reserved_names }
    }
}

impl Named for WellFormed {
    fn name() -> &'static str {
        "well-formed"
    }

    fn description() -> &'static str {
        "Check if the structure and control are well formed."
    }
}

impl Visitor for WellFormed {
    /// Check to see if any of the components use a reserved name or if the
    /// same name is bound by a group and a component.
    fn start(&mut self, comp: &mut Component, _x: &Context) -> VisResult {
        for (_, node) in comp.structure.component_iterator() {
            if self.reserved_names.contains(&node.name.id) {
                return Err(Error::ReservedName(node.name.clone()));
            }

            // If this is a cell, check for clash with group name.
            if let NodeData::Cell(_) = &node.data {
                if comp.structure.groups.contains_key(&Some(node.name.clone()))
                {
                    return Err(Error::AlreadyBound(
                        node.name.clone(),
                        "group".to_string(),
                    ));
                }
            }
        }
        Ok(Action::Continue)
    }

    /// Check to see if all groups mentioned in the control are defined.
    fn start_enable(
        &mut self,
        s: &Enable,
        comp: &mut Component,
        _x: &Context,
    ) -> VisResult {
        let st = &comp.structure;
        if !st.groups.contains_key(&Some(s.comp.clone())) {
            return Err(Error::UndefinedGroup(s.comp.clone()));
        }
        Ok(Action::Continue)
    }
}
