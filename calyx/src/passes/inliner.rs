use crate::{
    frontend::library::ast::LibrarySignatures,
    ir::{
        analysis::Analysis,
        traversal::{Action, Named, VisResult, Visitor},
        Assignment, Component, Control, Guard, Port,
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
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        // get the only group in the enable
        let top_level = match &*comp.control.borrow() {
            Control::Enable(en) => Rc::clone(&en.group),
            _ => panic!("need single enable"),
        };

        // add top_level[go] = this.go
        let go_asgn = Assignment {
            src: comp.signature.borrow().get("go"),
            dst: top_level.borrow().get("go"),
            guard: None,
        };
        // add this.done = top_level[done]
        let done_asgn = Assignment {
            src: top_level.borrow().get("done"),
            dst: comp.signature.borrow().get("done"),
            guard: None,
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
            // replace reads from a hole with the value in the map
            for asgn in &mut group.assignments {
                asgn.guard.as_mut().map(|guard| {
                    guard.for_each(&|port| {
                        if port.borrow().is_hole() {
                            map[&port.borrow().key()].clone()
                        } else {
                            Guard::Port(port)
                        }
                    })
                });
            }
        }
        // remove edges that write to a hole
        comp.continuous_assignments
            .retain(|asgn| !asgn.dst.borrow().is_hole());
        // replace reads from a hole with the value in the map
        for asgn in &mut comp.continuous_assignments {
            asgn.guard.as_mut().map(|guard| {
                guard.for_each(&|port| {
                    if port.borrow().is_hole() {
                        map[&port.borrow().key()].clone()
                    } else {
                        Guard::Port(port)
                    }
                })
            });
        }

        // remove group from control
        Ok(Action::Change(Control::empty()))
    }
}
