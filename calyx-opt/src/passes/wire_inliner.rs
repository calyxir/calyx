use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use ir::{build_assignments, guard, structure, LibrarySignatures};
use ir::{Nothing, RRC};
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

fn rewrite(map: &HoleMapping, port: &RRC<ir::Port>) -> Option<RRC<ir::Cell>> {
    if let ir::PortParent::Group(g) = &port.borrow().parent {
        let (go, done) = &map[&g.upgrade().borrow().name()];
        let cell = if port.borrow().name == "go" { go } else { done };
        Some(Rc::clone(cell))
    } else {
        None
    }
}

fn rewrite_assign(map: &HoleMapping, assign: &mut ir::Assignment<Nothing>) {
    if let Some(cell) = rewrite(map, &assign.dst) {
        assign.dst = cell.borrow().get("in");
    }
    if let Some(cell) = rewrite(map, &assign.src) {
        assign.src = cell.borrow().get("out");
    }
    assign.guard.for_each(&mut |port| {
        rewrite(map, &port)
            .map(|cell| ir::Guard::port(cell.borrow().get("out")))
    });
}

fn rewrite_guard(map: &HoleMapping, guard: &mut ir::Guard<Nothing>) {
    match guard {
        ir::Guard::True | ir::Guard::Info(_) => (),
        // update the port of a port guard to read from appropriate
        // group wire, if it is dependent on a group's port in the first place
        ir::Guard::Port(p) => {
            if let Some(cell) = rewrite(map, p) {
                guard.update(|_| ir::Guard::port(cell.borrow().get("out")));
            }
        }
        // update the ports of a port-comparison guard to read from appropriate
        // group wire, if these are dependent on groups' ports in the first place
        ir::Guard::CompOp(_, p1, p2) => {
            if let Some(cell) = rewrite(map, p1) {
                let _ = std::mem::replace(p1, cell.borrow().get("out"));
            }
            if let Some(cell) = rewrite(map, p2) {
                let _ = std::mem::replace(p2, cell.borrow().get("out"));
            }
        }
        ir::Guard::Not(b) => {
            rewrite_guard(map, &mut *b);
        }
        ir::Guard::And(b1, b2) | ir::Guard::Or(b1, b2) => {
            rewrite_guard(map, &mut *b1);
            rewrite_guard(map, &mut *b2);
        }
    }
}

impl Visitor for WireInliner {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // trigger start of component FSM based on component's go signal
        // trigger done of component based on FSM's done signal
        let control_ref = Rc::clone(&comp.control);
        let control = control_ref.borrow();
        match &*control {
            ir::Control::FSMEnable(fsm_en) => {
                let this = Rc::clone(&comp.signature);
                let mut builder = ir::Builder::new(comp, sigs);
                let comp_fsm = &fsm_en.fsm;
                let this_go_port = this
                    .borrow()
                    .find_unique_with_attr(ir::NumAttr::Go)?
                    .unwrap();
                structure!(builder;
                    let one = constant(1, 1);
                );
                let fsm_done = guard!(comp_fsm["done"]);
                let assigns = build_assignments!(builder;
                    comp_fsm["start"] = ? this[this_go_port.borrow().name];
                    this["done"] = fsm_done ? one["out"];
                );
                comp.continuous_assignments.extend(assigns);
            }
            ir::Control::Empty(_) => {}
            _ => {
                return Err(calyx_utils::Error::malformed_control(format!(
                    "{}: Structure has more than one group",
                    Self::name()
                )));
            }
        }

        // assume static groups is empty
        let groups = comp.get_groups_mut().drain().collect_vec();
        let mut builder = ir::Builder::new(comp, sigs);
        // for each group, instantiate wires to hold its `go` and `done` signals.
        let hole_map: HoleMapping = groups
            .iter()
            .map(|gr| {
                let name = gr.borrow().name();
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

        comp.get_fsms_mut().iter_mut().for_each(|fsm| {
            // rewrite all assignments at each state within every fsm
            for assigns_at_state in fsm.borrow_mut().assignments.iter_mut() {
                for asgn in assigns_at_state.iter_mut() {
                    rewrite_assign(&hole_map, asgn);
                }
            }
            // rewrite all guards in transitions that depend on group port values
            for trans_at_state in fsm.borrow_mut().transitions.iter_mut() {
                if let ir::Transition::Conditional(cond_trans_at_state) =
                    trans_at_state
                {
                    for (cond_trans, _) in cond_trans_at_state {
                        rewrite_guard(&hole_map, cond_trans)
                    }
                }
            }
        });

        // rewrite all transitions as well

        comp.continuous_assignments
            .iter_mut()
            .for_each(|assign| rewrite_assign(&hole_map, assign));

        let mut group_assigns = groups
            .into_iter()
            .flat_map(|g| g.borrow_mut().assignments.drain(..).collect_vec())
            .collect_vec();

        comp.continuous_assignments.append(&mut group_assigns);

        // remove group from control
        Ok(Action::change(ir::Control::empty()))
    }
}
