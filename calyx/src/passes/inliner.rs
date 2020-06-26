use crate::lang::component::Component;
use crate::lang::{
    ast, context::Context, structure, structure_builder, structure_iter,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use ast::{Atom, Control, GuardExpr};
use itertools::Itertools;
use petgraph::graph::EdgeIndex;
use std::collections::HashMap;
use structure::{DataDirection, StructureGraph};
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

fn apply_inlining_map(
    guard: &GuardExpr,
    map: &HashMap<Atom, (EdgeIndex, Vec<GuardExpr>)>,
    inlined: &mut Vec<EdgeIndex>,
) -> Vec<GuardExpr> {
    match guard {
        GuardExpr::Atom(a) => map
            .get(a)
            .map(|(idx, guards)| {
                inlined.push(*idx);
                guards
                    .iter()
                    .flat_map(|g| apply_inlining_map(g, map, inlined))
                    .collect()
            })
            .unwrap_or_else(|| vec![guard.clone()]),
        GuardExpr::Not(a) => {
            // XXX(sam) this is not correct right now, we don't recurse or handle the general case
            match map.get(a) {
                Some((idx, guard)) if guard.len() == 1 => {
                    inlined.push(*idx);
                    match &guard[0] {
                        GuardExpr::Atom(a) => vec![GuardExpr::Not(a.clone())],
                        _ => unimplemented!(
                            "ahh this will be annoying and break everything"
                        ),
                    }
                }
                Some(_) => unimplemented!(
                    "ahh this will be annoying and break everything"
                ),
                None => vec![guard.clone()],
            }
        }
        _ => vec![guard.clone()],
    }
}

/// Given a StructureGraph `st`, this function inlines assignments
/// to `x[hole]` into any uses of `x[hole]` in a GuardExpr. This works
/// in 2 passes over the edges. The first pass only considers assignments
/// into `x[hole]` and builds up a mapping from `x[hole] -> (edge index, guards)`
/// The second pass considers every edge and uses the map to replace every instance
/// of `x[hole]` in a guard with `guards`.
fn inline_hole(st: &mut StructureGraph, hole: String) {
    // a mapping from atoms -> (edge index, guard expressions)
    let mut map: HashMap<Atom, (EdgeIndex, Vec<GuardExpr>)> = HashMap::new();

    // build up the mapping by mapping over all writes into holes
    for idx in st
        .edge_idx()
        .with_direction(DataDirection::Write)
        .with_node_type(NodeType::Hole)
        .with_port(hole)
    {
        let ed = &st.graph[idx];
        let (src_idx, dest_idx) = st.endpoints(idx);
        let mut guards = ed.guards.clone();
        let atom = st.to_atom((src_idx, ed.src.port_name().clone()));
        // check if atom is just the constant 1, if so we don't need to put it in the guard
        if !matches!(
            atom,
            Atom::Num(ast::BitNum {
                width: 1, val: 1, ..
            })
        ) {
            guards.push(GuardExpr::Atom(atom));
        }
        // insert a mapping from hole to guards
        map.insert(
            st.to_atom((dest_idx, ed.dest.port_name().clone())),
            (idx, guards),
        );
    }

    // store for all the edges that were actually inlined
    let mut edges_inlined: Vec<EdgeIndex> = vec![];

    // iterate over all edges to replace holes with an expression
    for edidx in st.edge_idx().detach() {
        let mut ed = &mut st.graph[edidx];
        // for each guard, if the atom is in `map` then replace it with the expression
        // found there and add the corresponding edge index to `edges_inlined`
        ed.guards = ed
            .guards
            .iter()
            // we use flat map because a single hole may be replaced with multiple expressions in general
            .flat_map(|guard| {
                apply_inlining_map(guard, &map, &mut edges_inlined)
            })
            // redundant expressions are useless when everything is just anded together
            .unique()
            .collect()
    }

    // remove all the edges that have been inlined
    for idx in edges_inlined {
        st.remove_edge(idx);
    }

    // flatten groups (maybe temporary)
    for idx in st.edge_idx().detach() {
        st.graph[idx].group = None;
    }
    st.groups = HashMap::new();
    st.groups.insert(None, st.edge_idx().collect());
}

impl Visitor for Inliner {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        let st = &mut comp.structure;

        // inline `go` holes first because `done` holes may reference them
        inline_hole(st, "go".to_string());
        inline_hole(st, "done".to_string());

        comp.control = Control::empty();

        // This pass doesn't modify any control.
        Ok(Action::Stop)
    }
}
