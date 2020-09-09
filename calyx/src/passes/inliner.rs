use crate::lang::component::Component;
use crate::lang::{
    ast, context::Context, structure, structure_builder, structure_iter,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use ast::{Atom, Control, GuardExpr};
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

/// A mapping from destination ports to the guard expressions that write
/// into them.
type GuardMap = HashMap<Atom, GuardExpr>;

/// Walks the `GuardExpr` ast and replaces Atoms `a` with
/// it's corresponding entry in `map` if one exists.
fn tree_walk(guard: GuardExpr, map: &GuardMap) -> GuardExpr {
    match guard {
        GuardExpr::Atom(a) => map
            .get(&a)
            .map(|g| tree_walk(g.clone(), &map))
            .unwrap_or(GuardExpr::Atom(a)),
        GuardExpr::Not(inner) => {
            GuardExpr::Not(Box::new(tree_walk(*inner, &map)))
        }
        GuardExpr::And(bs) => GuardExpr::and_vec(
            bs.into_iter().map(|b| tree_walk(b, &map)).collect(),
        ),
        GuardExpr::Or(bs) => GuardExpr::or_vec(
            bs.into_iter().map(|b| tree_walk(b, &map)).collect(),
        ),
        GuardExpr::Eq(left, right) => GuardExpr::Eq(
            Box::new(tree_walk(*left, &map)),
            Box::new(tree_walk(*right, &map)),
        ),
        GuardExpr::Neq(left, right) => GuardExpr::Neq(
            Box::new(tree_walk(*left, &map)),
            Box::new(tree_walk(*right, &map)),
        ),
        GuardExpr::Gt(left, right) => GuardExpr::Gt(
            Box::new(tree_walk(*left, &map)),
            Box::new(tree_walk(*right, &map)),
        ),
        GuardExpr::Lt(left, right) => GuardExpr::Lt(
            Box::new(tree_walk(*left, &map)),
            Box::new(tree_walk(*right, &map)),
        ),
        GuardExpr::Geq(left, right) => GuardExpr::Geq(
            Box::new(tree_walk(*left, &map)),
            Box::new(tree_walk(*right, &map)),
        ),
        GuardExpr::Leq(left, right) => GuardExpr::Leq(
            Box::new(tree_walk(*left, &map)),
            Box::new(tree_walk(*right, &map)),
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
    // a mapping from atoms (dst) -> Vec<GuardExpr> (sources) that write
    // into the atom.
    let mut multi_guard_map: HashMap<Atom, Vec<GuardExpr>> = HashMap::new();

    // build up the mapping by mapping over all writes into holes
    for idx in st
        .edge_idx()
        .with_direction(DataDirection::Write)
        .with_node_type(NodeType::Hole)
        .with_port(hole.clone())
    {
        let ed = &st.get_edge(idx);
        let (src_idx, dest_idx) = st.endpoints(idx);
        let guard_opt = ed.guard.clone();
        let atom = st.to_atom((src_idx, ed.src.port_name().clone()));

        // ASSUMES: The values being assigned into holes are one-bit.
        // Transform `x[hole] = src` into `x[hole] = src ? 1`;
        let guard = match guard_opt {
            Some(g) => g & GuardExpr::Atom(atom),
            None => GuardExpr::Atom(atom),
        };

        // Add this guard to the guard map.
        let key = st.to_atom((dest_idx, ed.dest.port_name().clone()));
        let guards = multi_guard_map.entry(key).or_insert_with(Vec::new);
        guards.push(guard);
    }

    // Create a GuardMap but creating guard edges that `or` together
    // individual guards collected in multi_guard_map.
    let guard_map: GuardMap = multi_guard_map
        .into_iter()
        .map(|(k, v)| (k, GuardExpr::or_vec(v)))
        .collect();

    // iterate over all edges to replace holes with an expression
    for ed_idx in st.edge_idx().detach() {
        let mut ed_data = st.get_edge_mut(ed_idx);
        // for the guard, recur down the guard ast and replace any leaves with
        // the expression in `guard_map` adding the corresponding edges to `edges_inlined`
        ed_data.guard = ed_data
            .guard
            .as_ref()
            .map(|guard| tree_walk(guard.clone(), &guard_map));
    }

    // remove all the edges that have no reads from them.
    for idx in st
        .edge_idx()
        .with_direction(DataDirection::Write)
        .with_node_type(NodeType::Hole)
        .with_port(hole)
        .detach()
    {
        st.remove_edge(idx);
    }

    // flatten groups (maybe temporary)
    for idx in st.edge_idx().detach() {
        st.get_edge_mut(idx).group = None;
    }
    st.groups = HashMap::new();
    st.groups
        .insert(None, (HashMap::new(), st.edge_idx().collect()));
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
