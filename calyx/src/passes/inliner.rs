use crate::{
    frontend::library::ast::LibrarySignatures,
    ir::{
        analysis::Analysis,
        traversal::{Action, Named, VisResult, Visitor},
        Assignment, Builder, Component, Control, Guard, Port,
    },
    utils::Keyable,
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

/// Find the closure of everything that writes into `hole`
/// and merge them into a single guard.
fn write_closure(analysis: &Analysis, hole: &Port) -> Guard {
    analysis.writes_to(hole).fold(Guard::True, |acc, port| {
        if port.borrow().is_hole() {
            acc.and(write_closure(&analysis, &port.borrow()))
        } else {
            acc.and(Guard::Port(port))
        }
    })
}

impl Visitor for Inliner {
    fn start(
        &mut self,
        comp: &mut Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        // get the only group in the enable
        let top_level = match &*comp.control.borrow() {
            Control::Enable(en) => Rc::clone(&en.group),
            _ => panic!("need single enable"),
        };

        // borrow these first so that we can use the builder
        let go_sig = comp.signature.borrow().get("go");
        let done_rig = comp.signature.borrow().get("done");

        // make a builder for constructing constants
        let mut builder = Builder::from(comp, sigs, false);

        // add top_level[go] = this.go
        let go_asgn = Assignment {
            src: builder.add_constant(1, 1).borrow().get("out"),
            dst: top_level.borrow().get("go"),
            guard: Some(Guard::Port(go_sig)),
        };
        // add this.done = top_level[done]
        let done_asgn = Assignment {
            src: builder.add_constant(1, 1).borrow().get("out"),
            dst: done_rig,
            guard: Some(Guard::Port(top_level.borrow().get("done"))),
        };
        comp.continuous_assignments.push(go_asgn);
        comp.continuous_assignments.push(done_asgn);

        // construct analysis graph and find sub-graph of all edges that include a hole
        let analysis = Analysis::from(&comp);
        let subgraph = analysis
            .clone()
            .edge_induced_subgraph(|src, dst| src.is_hole() || dst.is_hole());

        // if subgraph has cycles, error out
        if subgraph.has_cycles() {
            // XXX use topo sort to find where the cycle is
            panic!("uh oh, hole cycle")
        }

        // map of holes to their guard expressions
        let mut map = HashMap::new();
        for group in &comp.groups {
            // compute write closure
            for hole in &group.borrow().holes {
                map.insert(
                    hole.borrow().key(),
                    write_closure(&analysis, &hole.borrow()),
                );
            }

            let mut group = group.borrow_mut();
            // remove edges that write to a hole
            group
                .assignments
                .retain(|asgn| !asgn.dst.borrow().is_hole());

            let mut assignments: Vec<_> = group.assignments.drain(..).collect();
            // replace reads from a hole with the value in the map
            for asgn in &mut assignments {
                asgn.guard.as_mut().map(|guard| {
                    guard.for_each(&|port| {
                        if port.is_hole() {
                            Some(map[&port.key()].clone())
                        } else {
                            None
                        }
                    })
                });
            }
            group.assignments = assignments;
        }
        // remove edges that write to a hole
        comp.continuous_assignments
            .retain(|asgn| !asgn.dst.borrow().is_hole());

        let mut assignments: Vec<_> =
            comp.continuous_assignments.drain(..).collect();
        // replace reads from a hole with the value in the map
        for asgn in &mut assignments {
            asgn.guard.as_mut().map(|guard| {
                guard.for_each(&|port| {
                    if port.is_hole() {
                        Some(map[&port.key()].clone())
                    } else {
                        None
                    }
                })
            });
        }
        comp.continuous_assignments = assignments;

        // remove group from control
        Ok(Action::Change(Control::empty()))
    }
}
