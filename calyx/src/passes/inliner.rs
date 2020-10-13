use crate::{
    frontend::library::ast::LibrarySignatures,
    ir::{
        traversal::{Action, Named, VisResult, Visitor},
        Component, Guard, Id, Port, RRC,
    },
};
use std::{collections::HashMap, rc::Rc};

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
    fn start(
        &mut self,
        comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        // writes into [go]
        let mut go_writes: HashMap<Id, Guard> = HashMap::new();
        let mut done_writes: HashMap<Id, Guard> = HashMap::new();

        // gather all writes into [go] and [done]
        for group in &comp.groups {
            let group = group.borrow();
            for asgn in &group.assignments {
                let port = asgn.dst.borrow();
                // save writes into go
                if port.is_hole() && port.name == "go" {
                    let guard = asgn
                        .guard
                        .as_ref()
                        .map_or(Guard::Port(Rc::clone(&asgn.src)), |g| {
                            g.and(Guard::Port(Rc::clone(&asgn.src)))
                        });
                    // use parent port name because writes to go are only
                    // allows in other groups
                    go_writes.insert(port.get_parent_name(), guard);
                }
                // save writes into done
                if port.is_hole() && port.name == "done" {
                    let guard = asgn
                        .guard
                        .as_ref()
                        .map_or(Guard::Port(Rc::clone(&asgn.src)), |g| {
                            g.and(Guard::Port(Rc::clone(&asgn.src)))
                        });
                    // use group name because writes to done are only allowed
                    // in this group
                    done_writes.insert(group.name.clone(), guard);
                }
            }
        }

        // replace go/done holes with their writes
        for group in &comp.groups {
            let mut group = group.borrow_mut();
            let new_guard = go_writes.remove(&group.name);
            for asgn in &mut group.assignments {
                let src = asgn.src.borrow();
                // go holes
                if src.is_hole() && src.name == "go" {
                    println!("READ!")
                }
                asgn.guard.as_mut().map(|guard| {
                    guard.for_each(&|port: RRC<Port>| {
                        let port_name = port.borrow().name.clone();
                        if port_name == "go" {
                            match &new_guard {
                                Some(g) => g.clone(),
                                None => panic!("No writes into [go] hole."),
                            }
                        } else if port_name == "done" {
                            match go_writes
                                .get(&port.borrow().get_parent_name())
                            {
                                Some(g) => g.clone(),
                                None => {
                                    panic!("No writes into this [done] hole.")
                                }
                            }
                        } else {
                            Guard::Port(port)
                        }
                    })
                });

                // done holes
            }
        }

        // /// A mapping from destination ports to the guard expressions that write
        // /// into them.
        // type GuardMap = HashMap<Atom, GuardExpr>;

        // /// Walks the Symbol’s value as variable is void: GuardExpr ast and replaces Atoms Symbol’s value as variable is void: a with
        // /// it's corresponding entry in Symbol’s value as variable is void: map if one exists.
        // // fn tree_walk(guard: GuardExpr, map: &GuardMap) -> GuardExpr {
        // //     match guard {
        // //         GuardExpr::Atom(a) => map
        // //             .get(&a)
        // //             .map(|g| tree_walk(g.clone(), &map))
        // //             .unwrap_or(GuardExpr::Atom(a)),
        // //         GuardExpr::Not(inner) => {
        // //             GuardExpr::Not(Box::new(tree_walk(*inner, &map)))
        // //         }
        // //         GuardExpr::And(bs) => GuardExpr::and_vec(
        // //             bs.into_iter().map(|b| tree_walk(b, &map)).collect(),
        // //         ),
        // //         GuardExpr::Or(bs) => GuardExpr::or_vec(
        // //             bs.into_iter().map(|b| tree_walk(b, &map)).collect(),
        // //         ),
        // //         GuardExpr::Eq(left, right) => GuardExpr::Eq(
        // //             Box::new(tree_walk(*left, &map)),
        // //             Box::new(tree_walk(*right, &map)),
        // //         ),
        // //         GuardExpr::Neq(left, right) => GuardExpr::Neq(
        // //             Box::new(tree_walk(*left, &map)),
        // //             Box::new(tree_walk(*right, &map)),
        // //         ),
        // //         GuardExpr::Gt(left, right) => GuardExpr::Gt(
        // //             Box::new(tree_walk(*left, &map)),
        // //             Box::new(tree_walk(*right, &map)),
        // //         ),
        // //         GuardExpr::Lt(left, right) => GuardExpr::Lt(
        // //             Box::new(tree_walk(*left, &map)),
        // //             Box::new(tree_walk(*right, &map)),
        // //         ),
        // //         GuardExpr::Geq(left, right) => GuardExpr::Geq(
        // //             Box::new(tree_walk(*left, &map)),
        // //             Box::new(tree_walk(*right, &map)),
        // //         ),
        // //         GuardExpr::Leq(left, right) => GuardExpr::Leq(
        // //             Box::new(tree_walk(*left, &map)),
        // //             Box::new(tree_walk(*right, &map)),
        // //         ),
        // //     }
        // // }

        // /// Given a StructureGraph Symbol’s value as variable is void: st, this function inlines assignments
        // /// to Symbol’s value as variable is void: x into any uses of Symbol’s value as variable is void: x in a GuardExpr. This works
        // /// in 2 passes over the edges. The first pass only considers assignments
        // /// into Symbol’s value as variable is void: x and builds up a mapping from Symbol’s value as variable is void: x
        // /// The second pass considers every edge and uses the map to replace every instance
        // /// of Symbol’s value as variable is void: x in a guard with Symbol’s value as variable is void: guards.
        // fn inline_hole(st: &mut StructureGraph, hole: String) {
        //     // a mapping from atoms (dst) -> Vec<GuardExpr> (sources) that write
        //     // into the atom.
        //     let mut multi_guard_map: HashMap<Atom, Vec<GuardExpr>> =
        //         HashMap::new();

        //     // build up the mapping by mapping over all writes into holes
        //     for idx in st
        //         .edge_idx()
        //         .with_direction(DataDirection::Write)
        //         .with_node_type(NodeType::Hole)
        //         .with_port(hole.clone())
        //     {
        //         let ed = &st.get_edge(idx);
        //         let (src_idx, dest_idx) = st.endpoints(idx);
        //         let guard_opt = ed.guard.clone();
        //         let atom = st.to_atom((src_idx, ed.src.port_name().clone()));

        //         // ASSUMES: The values being assigned into holes are one-bit.
        //         // Transform Symbol’s value as variable is void: x into Symbol’s value as variable is void: x;
        //         let guard = match guard_opt {
        //             Some(g) => g & GuardExpr::Atom(atom),
        //             None => GuardExpr::Atom(atom),
        //         };

        //         // Add this guard to the guard map.
        //         let key = st.to_atom((dest_idx, ed.dest.port_name().clone()));
        //         let guards =
        //             multi_guard_map.entry(key).or_insert_with(Vec::new);
        //         guards.push(guard);
        //     }

        //     // Create a GuardMap but creating guard edges that Symbol’s value as variable is void: or together
        //     // individual guards collected in multi_guard_map.
        //     let guard_map: GuardMap = multi_guard_map
        //         .into_iter()
        //         .map(|(k, v)| (k, GuardExpr::or_vec(v)))
        //         .collect();

        //     // iterate over all edges to replace holes with an expression
        //     for ed_idx in st.edge_idx().detach() {
        //         let mut ed_data = st.get_edge_mut(ed_idx);
        //         // for the guard, recur down the guard ast and replace any leaves with
        //         // the expression in Symbol’s value as variable is void: guard_map adding the corresponding edges to Symbol’s value as variable is void: edges_inlined
        //         ed_data.guard = ed_data
        //             .guard
        //             .as_ref()
        //             .map(|guard| tree_walk(guard.clone(), &guard_map));
        //     }

        //     // remove all the edges that have no reads from them.
        //     for idx in st
        //         .edge_idx()
        //         .with_direction(DataDirection::Write)
        //         .with_node_type(NodeType::Hole)
        //         .with_port(hole)
        //         .detach()
        //     {
        //         st.remove_edge(idx);
        //     }

        //     // flatten groups (maybe temporary)
        //     for idx in st.edge_idx().detach() {
        //         st.get_edge_mut(idx).group = None;
        //     }
        //     st.groups = HashMap::new();
        //     st.groups
        //         .insert(None, (HashMap::new(), st.edge_idx().collect()));
        // }
        // let st = &mut comp.structure;

        // // inline Symbol’s value as variable is void: go holes first because Symbol’s value as variable is void: done holes may reference them
        // inline_hole(st, "go".to_string());
        // inline_hole(st, "done".to_string());

        // comp.control = Control::empty();

        // This pass doesn't modify any control.
        Ok(Action::Stop)
    }
}
