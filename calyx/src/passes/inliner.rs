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

type GuardMap = HashMap<Atom, (EdgeIndex, GuardExpr)>;

/// Walks the `GuardExpr` ast and replaces Atoms `a` with
/// it's corresponding entry in `map` if one exists.
fn tree_walk(
    guard: GuardExpr,
    map: &GuardMap,
    edges_inlined: &mut Vec<EdgeIndex>,
) -> GuardExpr {
    match guard {
        GuardExpr::Atom(a) => map
            .get(&a)
            .map(|(idx, g)| {
                edges_inlined.push(*idx);
                tree_walk(g.clone(), &map, edges_inlined)
            })
            .unwrap_or(GuardExpr::Atom(a)),
        GuardExpr::Not(inner) => {
            GuardExpr::Not(Box::new(tree_walk(*inner, &map, edges_inlined)))
        }
        GuardExpr::And(left, right) => GuardExpr::And(
            Box::new(tree_walk(*left, &map, edges_inlined)),
            Box::new(tree_walk(*right, &map, edges_inlined)),
        ),
        GuardExpr::Or(left, right) => GuardExpr::Or(
            Box::new(tree_walk(*left, &map, edges_inlined)),
            Box::new(tree_walk(*right, &map, edges_inlined)),
        ),
        GuardExpr::Eq(left, right) => GuardExpr::Eq(
            Box::new(tree_walk(*left, &map, edges_inlined)),
            Box::new(tree_walk(*right, &map, edges_inlined)),
        ),
        GuardExpr::Neq(left, right) => GuardExpr::Neq(
            Box::new(tree_walk(*left, &map, edges_inlined)),
            Box::new(tree_walk(*right, &map, edges_inlined)),
        ),
        GuardExpr::Gt(left, right) => GuardExpr::Gt(
            Box::new(tree_walk(*left, &map, edges_inlined)),
            Box::new(tree_walk(*right, &map, edges_inlined)),
        ),
        GuardExpr::Lt(left, right) => GuardExpr::Lt(
            Box::new(tree_walk(*left, &map, edges_inlined)),
            Box::new(tree_walk(*right, &map, edges_inlined)),
        ),
        GuardExpr::Geq(left, right) => GuardExpr::Geq(
            Box::new(tree_walk(*left, &map, edges_inlined)),
            Box::new(tree_walk(*right, &map, edges_inlined)),
        ),
        GuardExpr::Leq(left, right) => GuardExpr::Leq(
            Box::new(tree_walk(*left, &map, edges_inlined)),
            Box::new(tree_walk(*right, &map, edges_inlined)),
        ),
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
    let mut guard_map: GuardMap = HashMap::new();

    // build up the mapping by mapping over all writes into holes
    for idx in st
        .edge_idx()
        .with_direction(DataDirection::Write)
        .with_node_type(NodeType::Hole)
        .with_port(hole)
    {
        let ed = &st.get_edge(idx);
        let (src_idx, dest_idx) = st.endpoints(idx);
        let mut guard_opt = ed.guard.clone();
        let atom = st.to_atom((src_idx, ed.src.port_name().clone()));
        // if atom is just the constant 1, we don't need to put it in the guard
        // if !matches!(
        //     atom,
        //     Atom::Num(ast::BitNum {
        //         width: 1, val: 1, ..
        //     })
        // ) {
        // }
        guard_opt = Some(match guard_opt {
            Some(g) => g & GuardExpr::Atom(atom),
            None => GuardExpr::Atom(atom),
        });
        // insert a mapping from hole to guards
        guard_opt.map(|guard| {
            guard_map.insert(
                st.to_atom((dest_idx, ed.dest.port_name().clone())),
                (idx, guard),
            )
        });
    }

    // store for all the edges that were actually inlined so that we can remove them later
    let mut edges_inlined: Vec<EdgeIndex> = vec![];

    // iterate over all edges to replace holes with an expression
    for edidx in st.edge_idx().detach() {
        let mut ed_data = st.get_edge_mut(edidx);
        // for the guard, recurse down the guard ast and replace any leaves with
        // the expression in `guard_map` adding the corresponding edges to `edges_inlined`
        ed_data.guard = ed_data.guard.as_ref().map(|guard| {
            tree_walk(guard.clone(), &guard_map, &mut edges_inlined)
        });
    }

    // remove all the edges that have been inlined
    for idx in edges_inlined.iter().unique() {
        st.remove_edge(*idx);
    }

    // flatten groups (maybe temporary)
    for idx in st.edge_idx().detach() {
        st.get_edge_mut(idx).group = None;
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
