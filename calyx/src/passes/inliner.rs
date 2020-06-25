use crate::lang::component::Component;
use crate::lang::{
    ast, context::Context, structure, structure_builder, structure_iter,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use ast::{Atom, GuardExpr, Port};
use petgraph::graph::EdgeIndex;
use std::collections::HashMap;
use structure::DataDirection;
use structure_builder::ASTBuilder;
use structure_iter::NodeType;

#[derive(Default)]
pub struct Inliner;

impl Named for Inliner {
    fn name() -> &'static str {
        "hole-inliner"
    }

    fn description() -> &'static str {
        "inlines holes"
    }
}

impl Visitor for Inliner {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        let st = &mut comp.structure;

        // a mapping from holes -> guard expressions
        let mut go_map: HashMap<Atom, Vec<GuardExpr>> = HashMap::new();

        // build up the mapping
        for idx in st
            .edge_idx()
            .with_direction(DataDirection::Write)
            .with_node_type(NodeType::Hole)
            .with_port("go".to_string())
            .detach()
        {
            let ed = &st.graph[idx];
            let (src_idx, dest_idx) = st.endpoints(idx);
            let mut guards = ed.guards.clone();
            let atom = st.to_atom((src_idx, ed.src.port_name().clone()));
            if !matches!(
                atom,
                Atom::Num(ast::BitNum {
                    width: 1, val: 1, ..
                })
            ) {
                guards.push(GuardExpr::Atom(atom));
            }
            go_map.insert(
                st.to_atom((dest_idx, ed.dest.port_name().clone())),
                guards,
            );
        }

        for edidx in st.edge_idx().detach() {
            let mut ed = &mut st.graph[edidx];
            ed.guards = ed
                .guards
                .iter()
                .flat_map(|guard| match guard {
                    GuardExpr::Atom(a) => match go_map.get(a) {
                        Some(x) => x.clone(),
                        None => vec![guard.clone()],
                    },
                    GuardExpr::Not(_) => unimplemented!(
                        "ahh this will be annoying and break everything"
                    ),
                    _ => vec![guard.clone()],
                })
                .collect()
        }

        let mut done_map: HashMap<Atom, Vec<GuardExpr>> = HashMap::new();

        // build up the mapping
        for idx in st
            .edge_idx()
            .with_direction(DataDirection::Write)
            .with_node_type(NodeType::Hole)
            .with_port("done".to_string())
            .detach()
        {
            let ed = &st.graph[idx];
            let (src_idx, dest_idx) = st.endpoints(idx);
            let mut guards = ed.guards.clone();
            let atom = st.to_atom((src_idx, ed.src.port_name().clone()));
            if !matches!(
                atom,
                Atom::Num(ast::BitNum {
                    width: 1, val: 1, ..
                })
            ) {
                guards.push(GuardExpr::Atom(atom));
            }
            done_map.insert(
                st.to_atom((dest_idx, ed.dest.port_name().clone())),
                guards,
            );
        }

        for edidx in st.edge_idx().detach() {
            let mut ed = &mut st.graph[edidx];
            ed.guards = ed
                .guards
                .iter()
                .flat_map(|guard| match guard {
                    GuardExpr::Atom(a) => {
                        if done_map.contains_key(a) {
                            println!("{:#?} -> {:#?}", guard, a);
                        }
                        match done_map.get(a) {
                            Some(x) => x.clone(),
                            None => vec![guard.clone()],
                        }
                    }
                    GuardExpr::Not(_) => unimplemented!(
                        "ahh this will be annoying and break everything"
                    ),
                    _ => vec![guard.clone()],
                })
                .collect()
        }

        // This pass doesn't modify any control.
        Ok(Action::Stop)
    }
}
