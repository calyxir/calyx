use crate::traversal::{Action, Named, VisResult, Visitor};
use crate::{analysis::GraphAnalysis, passes::TopDownCompileControl};
use calyx_ir::{self as ir, structure, LibrarySignatures, RRC};
use calyx_utils::Error;
use ir::Nothing;
use std::{collections::HashMap, rc::Rc};

#[derive(Default)]
/// Removes all groups and inlines reads and writes from holes.
///
/// After running this pass, there are no groups left in the `wires` section
/// of the program.
/// All remaining wires are continuous assignments which can be transformed
/// into wires in a hardware description language.
pub struct HoleInliner;

impl Named for HoleInliner {
    fn name() -> &'static str {
        "hole-inliner"
    }

    fn description() -> &'static str {
        "inlines holes"
    }
}

type Store = HashMap<ir::Canonical, (RRC<ir::Port>, ir::Guard<ir::Nothing>)>;

/// Finds the 'fixed_point' of a map from Hole names to guards under the
/// inlining operation. The map contains entries like:
/// ```
/// A[go] -> some_thing & B[go] & !A[done]
/// B[go] -> C[go]
/// C[go] -> go
/// ...
/// ```
/// We want to transform this so that the guard expression for every
/// hole does not itself contain holes.
///
/// We compute the fixed point using a worklist algorithm.
/// Variables:
///  - `guard(x)`: refers to the guard of the hole `x`
///  - `worklist`: a queue that contains fully inlined guards that have not yet been inlined into other guards
///
/// Algorithm:
///  - `worklist` is initialized to be all the holes that contain no holes in their guards.
///  - while there are things in `worklist`:
///    - pop a hole, `H`, from `worklist`
///    - for every hole, `a` that reads from `H`
///      - replace all instances of `H` in `guard(a)` with `guard(H)`
///      - if no holes in `guard(a)`, add to `worklist`
fn fixed_point(graph: &GraphAnalysis, map: &mut Store) {
    // keeps track of next holes we can inline
    let mut worklist = Vec::new();

    // helper to check if a guard has holes
    let has_holes = |guard: &ir::Guard<Nothing>| {
        guard
            .all_ports()
            .iter()
            .map(|p| p.borrow().is_hole())
            .any(|e| e)
    };

    // initialize the worklist to have guards that have no holes
    for (key, (_, guard)) in map.iter() {
        if !has_holes(guard) {
            worklist.push(key.clone())
        }
    }

    while !worklist.is_empty() {
        let hole_key = worklist.pop().unwrap_or_else(|| unreachable!());
        let (hole, new_guard) = map[&hole_key].clone();

        // for every read from the hole
        for read in graph
            .reads_from(&hole.borrow())
            .filter(|p| p.borrow().is_hole())
        {
            // inline `hole_key` into `read`
            let key = read.borrow().canonical();
            map.entry(read.borrow().canonical())
                .and_modify(|(_, guard)| {
                    guard.for_each(&mut |port| {
                        if port.borrow().canonical() == hole_key {
                            Some(new_guard.clone())
                        } else {
                            None
                        }
                    })
                });
            // if done with this guard, add it to the worklist
            if !has_holes(&map[&key].1) {
                worklist.push(key)
            }
        }
    }
}

impl Visitor for HoleInliner {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // get the only group in the enable
        let top_level = match &*comp.control.borrow() {
            ir::Control::Empty(_) => return Ok(Action::Stop),
            ir::Control::Enable(en) => Rc::clone(&en.group),
            _ => return Err(Error::malformed_control(format!(
                    "{}: Control shoudl be a single enable. Try running `{}` before inlining.",
                    Self::name(),
                    TopDownCompileControl::name()))
            )
        };

        let this_comp = Rc::clone(&comp.signature);
        let mut builder = ir::Builder::new(comp, sigs);

        // add top_level[go] = this.go
        let mut asgns = vec![
            builder.build_assignment(
                top_level.borrow().get("go"),
                this_comp.borrow().get_unique_with_attr(ir::NumAttr::Go)?,
                ir::Guard::True,
            ),
            builder.build_assignment(
                this_comp.borrow().get_unique_with_attr(ir::NumAttr::Done)?,
                top_level.borrow().get("done"),
                ir::Guard::True,
            ),
        ];
        builder.component.continuous_assignments.append(&mut asgns);

        // construct analysis graph and find sub-graph of all edges that include a hole
        let analysis = GraphAnalysis::from(&*builder.component);
        let subgraph = analysis
            .edge_induced_subgraph(|src, dst| src.is_hole() || dst.is_hole());

        // if subgraph has cycles, error out
        if subgraph.has_cycles() {
            // XXX use topo sort to find where the cycle is
            return Err(Error::malformed_structure(
                "Cyclic hole definition.".to_string(),
            ));
        }

        // map of holes to their guard expressions
        let mut map: Store = HashMap::new();
        let mut assignments = vec![];
        for group in builder.component.get_groups().iter() {
            // remove all assignments from group, taking ownership
            let mut group = group.borrow_mut();
            assignments.append(&mut group.assignments.drain(..).collect());
        }

        assert!(
            builder.component.get_static_groups().is_empty(),
            "should have removed static groups when inlining holes"
        );

        // add the continuous assignment edges
        assignments.append(
            &mut builder.component.continuous_assignments.drain(..).collect(),
        );

        for asgn in &mut assignments {
            // if assignment writes into a hole, save it
            let dst = asgn.dst.borrow();
            if dst.is_hole() {
                map.entry(dst.canonical())
                    .and_modify(|(_, val)| {
                        // XXX: seems like unncessary clone
                        *val = val.clone().or(asgn
                            .guard
                            .clone()
                            .and(ir::Guard::port(Rc::clone(&asgn.src))));
                    })
                    .or_insert((
                        Rc::clone(&asgn.dst),
                        asgn.guard
                            .clone()
                            .and(ir::Guard::port(Rc::clone(&asgn.src))),
                    ));
            }
        }

        // find fixed point of map
        fixed_point(&subgraph, &mut map);

        // remove edges that write to a hole
        assignments.retain(|asgn| !asgn.dst.borrow().is_hole());

        // move direct reads from holes into the guard so they can be inlined
        //   e.g. s.in = G[go]; => s.in G[go] ? 1'b1;
        structure!(
            builder;
            let signal_on = constant(1, 1);
        );
        assignments.iter_mut().for_each(|asgn| {
            if asgn.src.borrow().is_hole() {
                let and_guard = ir::Guard::port(Rc::clone(&asgn.src));
                *asgn.guard &= and_guard;
                asgn.src = signal_on.borrow().get("out");
            }
        });

        // replace reads from a hole with the value in the map
        for asgn in &mut assignments {
            asgn.guard.for_each(&mut |port| {
                if port.borrow().is_hole() {
                    Some(map[&port.borrow().canonical()].1.clone())
                } else {
                    None
                }
            })
        }
        comp.continuous_assignments = assignments;

        // remove all groups
        comp.get_groups_mut().clear();
        comp.get_static_groups_mut().clear();

        // remove group from control
        Ok(Action::change(ir::Control::empty()))
    }
}
