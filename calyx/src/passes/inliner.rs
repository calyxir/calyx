//! Removes all groups and inlines reads and writes from holes.
//! After running this pass, there are no groups left in the `wires` section
//! of the program.
//! All remaining wires are continuous assignments which can be transformed
//! into wires in a hardware description language.
use crate::{
    build_assignments,
    errors::Error,
    frontend::library::ast::LibrarySignatures,
    ir,
    ir::{
        analysis::GraphAnalysis,
        traversal::{Action, Named, VisResult, Visitor},
    },
    structure,
    utils::Keyable,
};
use ir::RRC;
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

fn fixed_point(
    graph: &GraphAnalysis,
    map: &mut HashMap<(ir::Id, ir::Id), (RRC<ir::Port>, ir::Guard)>,
) {
    // keeps track of next holes we can inline
    let mut worklist = Vec::new();

    let has_holes = |guard: &ir::Guard| {
        guard
            .all_ports()
            .iter()
            .map(|p| p.borrow().is_hole())
            .fold(false, |acc, e| acc || e)
    };

    // initialize the worklist to have guards that have no holes
    for (key, (_, guard)) in map.iter() {
        println!("key: {:?}", key);
        if !has_holes(&guard) {
            worklist.push(key.clone())
        }
    }

    while !worklist.is_empty() {
        let hole_key = worklist.pop().unwrap_or_else(|| unreachable!());
        let (hole, new_guard) = map[&hole_key].clone();

        println!("{:?}", worklist);

        for read in graph
            .reads_from(&hole.borrow())
            .filter(|p| p.borrow().is_hole())
        {
            let key = read.borrow().key();
            map.entry(read.borrow().key()).and_modify(|(_, guard)| {
                guard.for_each(&|port: &ir::Port| {
                    if port.key() == hole_key {
                        Some(new_guard.clone())
                    } else {
                        None
                    }
                })
            });
            if !has_holes(&map[&key].1) {
                worklist.push(key)
            }
        }
    }
}

impl Visitor for Inliner {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        // get the only group in the enable
        let top_level = match &*comp.control.borrow() {
            ir::Control::Enable(en) => Rc::clone(&en.group),
            _ => return Err(
                Error::MalformedControl(
                    "The hole inliner requires control to be a single enable. Try running `compile_control` before inlining.".to_string()
                )
            )
        };

        let this_comp = Rc::clone(&comp.signature);
        let mut builder = ir::Builder::from(comp, sigs, false);
        // make a builder for constructing constants. Introduce new scope
        // for builder so that it only holds its mutable reference to
        // `comp` for the duration of this scope

        // add top_level[go] = this.go
        let mut asgns = build_assignments!(
            builder;
            top_level["go"] = ? this_comp["go"];
            this_comp["done"] = ? top_level["done"];
        );
        builder.component.continuous_assignments.append(&mut asgns);

        // construct analysis graph and find sub-graph of all edges that include a hole
        let analysis = GraphAnalysis::from(&builder.component);
        let subgraph = analysis
            .edge_induced_subgraph(|src, dst| src.is_hole() || dst.is_hole());

        // if subgraph has cycles, error out
        if subgraph.has_cycles() {
            // XXX use topo sort to find where the cycle is
            return Err(Error::MalformedStructure(
                "Cyclic hole definition.".to_string(),
            ));
        }

        // map of holes to their guard expressions
        let mut map: HashMap<_, (RRC<ir::Port>, ir::Guard)> = HashMap::new();
        let mut assignments = vec![];
        for group in &builder.component.groups {
            // remove all assignments from group, taking ownership
            let mut group = group.borrow_mut();
            assignments.append(&mut group.assignments.drain(..).collect());
        }

        // add the continuous assignment edges
        assignments.append(
            &mut builder.component.continuous_assignments.drain(..).collect(),
        );

        for asgn in &mut assignments {
            // if assignment writes into a hole, save it
            let dst = asgn.dst.borrow();
            if dst.is_hole() {
                map.entry(dst.key())
                    .and_modify(|(_, val)| {
                        // seems like unncessary clone
                        *val = val
                            .clone()
                            .and(asgn.guard.clone())
                            .and(ir::Guard::Port(Rc::clone(&asgn.src)));
                    })
                    .or_insert((
                        Rc::clone(&asgn.dst),
                        asgn.guard
                            .clone()
                            .and(ir::Guard::Port(Rc::clone(&asgn.src))),
                    ));
            }
        }

        fixed_point(&subgraph, &mut map);

        // remove edges that write to a hole
        assignments.retain(|asgn| !asgn.dst.borrow().is_hole());

        // move direct reads from holes into the guard so they can be inlined
        //   e.g. s.in = G[go]; => s.in G[go] ? 1'b1;
        structure!(
            builder;
            let signal_on = constant(1, 1);
        );
        assignments.iter_mut().for_each(|mut asgn| {
            if asgn.src.borrow().is_hole() {
                asgn.guard = asgn
                    .guard
                    .clone()
                    .and(ir::Guard::Port(Rc::clone(&asgn.src)));
                asgn.src = signal_on.borrow().get("out");
            }
        });

        // replace reads from a hole with the value in the map
        for asgn in &mut assignments {
            asgn.guard.for_each(&|port| {
                if port.is_hole() {
                    Some(
                        map.get(&port.key())
                            .expect(&format!("{:?}", port.key()))
                            .1
                            .clone(),
                    )
                } else {
                    None
                }
            })
        }
        comp.continuous_assignments = assignments;

        // remove all groups
        comp.groups.clear();

        // remove group from control
        Ok(Action::Change(ir::Control::empty()))
    }
}
