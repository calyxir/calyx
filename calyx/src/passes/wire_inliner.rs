use crate::{
    ir::{self, LibrarySignatures},
    ir::{
        traversal::{Action, Named, VisResult, Visitor},
        CloneName,
    },
};
use ir::RRC;
use itertools::Itertools;
use std::{collections::HashMap, rc::Rc};

#[derive(Default)]
/// Alternate hole inliner that removes groups and group holes by instantiating
/// wires that hold the value for each signal.
pub struct WireInliner;

type HoleMapping = HashMap<ir::Id, (RRC<ir::Cell>, RRC<ir::Cell>)>;

impl Named for WireInliner {
    fn name() -> &'static str {
        "wire-inliner"
    }

    fn description() -> &'static str {
        "inlines holes using wires"
    }
}

fn rewrite_assign(map: &HoleMapping, assign: &mut ir::Assignment) {
    let rewrite = |port: &RRC<ir::Port>| -> Option<RRC<ir::Cell>> {
        if let ir::PortParent::Group(g) = &port.borrow().parent {
            let (go, done) = &map[g.upgrade().borrow().name()];
            let cell = if port.borrow().name == "go" { go } else { done };
            Some(Rc::clone(cell))
        } else {
            None
        }
    };

    if let Some(cell) = rewrite(&assign.dst) {
        assign.dst = cell.borrow().get("in");
    }
    if let Some(cell) = rewrite(&assign.src) {
        assign.src = cell.borrow().get("out");
    }
    assign.guard.for_each(&|port| {
        rewrite(&port).map(|cell| ir::Guard::port(cell.borrow().get("out")))
    });
}

impl Visitor for WireInliner {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        let groups = comp.groups.drain().collect_vec();
        let mut builder = ir::Builder::new(comp, sigs);
        // for each group, instantiate wires to hold its `go` and `done` signals.
        let hole_map: HoleMapping = groups
            .iter()
            .map(|gr| {
                let name = gr.clone_name();
                let go = builder.add_primitive(
                    format!("{}_go", name),
                    "std_wire",
                    &[1],
                );
                let done = builder.add_primitive(
                    format!("{}_done", name),
                    "std_wire",
                    &[1],
                );
                (name, (go, done))
            })
            .collect();

        // Rewrite all assignments first
        groups.iter().for_each(|gr| {
            // Detach assignment from the group first because the rewrite
            // method will try to borrow the underlying group.
            // This means that the group's borrow_mut cannot be active when
            // rewrite_assign is called.
            let mut assigns =
                gr.borrow_mut().assignments.drain(..).collect_vec();
            assigns
                .iter_mut()
                .for_each(|asgn| rewrite_assign(&hole_map, asgn));
            gr.borrow_mut().assignments = assigns;
        });

        let mut group_assigns = groups
            .into_iter()
            .flat_map(|g| g.borrow_mut().assignments.drain(..).collect_vec())
            .collect_vec();

        comp.continuous_assignments
            .iter_mut()
            .for_each(|assign| rewrite_assign(&hole_map, assign));

        comp.continuous_assignments.append(&mut group_assigns);

        // remove group from control
        Ok(Action::Change(ir::Control::empty()))
    }
}
