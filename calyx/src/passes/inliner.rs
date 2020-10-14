use crate::{
    errors::Error,
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
            _ => return Err(
                Error::MalformedControl(
                    "The hole inliner requires control to be a single enable. Try running `compile_control` before inlining.".to_string()
                )
            )
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
            return Err(Error::MalformedStructure(
                "Cyclic hole definition.".to_string(),
            ));
        }

        // map of holes to their guard expressions
        let mut map = HashMap::new();
        let mut assignments = vec![];
        for group in &comp.groups {
            // compute write closure
            for hole in &group.borrow().holes {
                map.insert(
                    hole.borrow().key(),
                    write_closure(&analysis, &hole.borrow()),
                );
            }

            // remove all assignments from group, taking ownership
            let mut group = group.borrow_mut();
            assignments.append(&mut group.assignments.drain(..).collect());
        }

        // add the continuous assignment edges
        assignments
            .append(&mut comp.continuous_assignments.drain(..).collect());

        // remove edges that write to a hole
        assignments.retain(|asgn| !asgn.dst.borrow().is_hole());

        // move direct reads from holes into the guard so they can be inlined
        let mut builder = Builder::from(comp, sigs, false);
        assignments.iter_mut().for_each(|mut asgn| {
            if asgn.src.borrow().is_hole() {
                asgn.guard = Some(match &asgn.guard {
                    Some(g) => g.and(Guard::Port(Rc::clone(&asgn.src))),
                    None => Guard::Port(Rc::clone(&asgn.src)),
                });
                asgn.src = builder.add_constant(1, 1).borrow().get("out");
            }
        });

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

        // remove all groups
        comp.groups = vec![];

        // remove group from control
        Ok(Action::Change(Control::empty()))
    }
}
