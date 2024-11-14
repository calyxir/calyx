use std::collections::HashMap;

use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, BoolAttr, Guard, Nothing};
use calyx_utils::CalyxResult;

/// Adds probe wires to each group to detect when a group is active.
/// Used by the profiler.
pub struct ProfilerInstrumentation {
    // map from group to invocations
    group_map: HashMap<ir::Id, Vec<(ir::Id, ir::Guard<Nothing>)>>,
}

impl Named for ProfilerInstrumentation {
    fn name() -> &'static str {
        "profiler-instrumentation"
    }

    fn description() -> &'static str {
        "Add instrumentation for profiling"
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![]
    }
}

impl ConstructVisitor for ProfilerInstrumentation {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        Ok(ProfilerInstrumentation {
            group_map: HashMap::new(),
        })
    }

    fn clear_data(&mut self) {}
}

impl Visitor for ProfilerInstrumentation {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut acc = 0;
        let comp_name = comp.name;
        // iterate and check whether any groups invoke other groups
        for group_ref in comp.groups.iter() {
            let group = &group_ref.borrow();
            for assigment_ref in group.assignments.iter() {
                let dst_borrow = assigment_ref.dst.borrow();
                if let ir::PortParent::Group(parent_group_ref) =
                    &dst_borrow.parent
                {
                    if dst_borrow.name == "go" {
                        let done_port_ref =
                            parent_group_ref.upgrade().borrow().get("done");
                        // found an invocation of go
                        // FIXME: guard needs to be anded with the child group not being done
                        let invoked_group_name =
                            parent_group_ref.upgrade().borrow().name();
                        let guard = *(assigment_ref.guard.clone());
                        let combined_guard: Guard<Nothing> = Guard::and(
                            guard,
                            Guard::Not(Box::new(Guard::port(
                                done_port_ref.clone(),
                            ))),
                        );
                        match self.group_map.get_mut(&invoked_group_name) {
                            Some(vec_ref) => {
                                vec_ref.push((group.name(), combined_guard))
                            }
                            None => {
                                self.group_map.insert(
                                    invoked_group_name,
                                    vec![(group.name(), combined_guard)],
                                );
                            }
                        }
                        acc += 1; // really sad hack
                    }
                }
            }
        }
        // build probe and assignments for every group
        let mut builder = ir::Builder::new(comp, sigs);
        let mut group_name_assign_and_cell = Vec::with_capacity(acc);
        {
            for (invoked_group_name, parent_groups) in self.group_map.iter() {
                for (parent_group, guard) in parent_groups.iter() {
                    let probe_cell_name = format!(
                        "{}__{}__{}_probe",
                        invoked_group_name, parent_group, comp_name
                    );
                    let probe_cell = builder.add_primitive(
                        probe_cell_name,
                        "std_wire",
                        &[1],
                    );
                    probe_cell.borrow_mut().add_attribute(BoolAttr::Control, 1);
                    probe_cell
                        .borrow_mut()
                        .add_attribute(BoolAttr::Protected, 1);
                    let one = builder.add_constant(1, 1);
                    // FIXME: the assignment needs to take on the guard of the assignment and not the child group being done
                    let probe_asgn: ir::Assignment<Nothing> = builder
                        .build_assignment(
                            probe_cell.borrow().get("in"),
                            one.borrow().get("out"),
                            guard.clone(),
                        );
                    group_name_assign_and_cell.push((
                        parent_group.clone(),
                        probe_asgn,
                        probe_cell,
                    ));
                }
            }
        }
        // ugh so ugly
        for group in comp.groups.iter() {
            for (group_name, asgn, cell) in group_name_assign_and_cell.iter() {
                if group.borrow().name() == group_name {
                    group.borrow_mut().assignments.push(asgn.clone());
                    comp.cells.add(cell.to_owned());
                }
            }
        }
        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut calyx_ir::Enable,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let invoked_group_name = s.group.borrow().name();
        let comp_name = comp.name;
        match self.group_map.get_mut(&invoked_group_name) {
            Some(vec_ref) => vec_ref.push((comp_name, calyx_ir::Guard::True)),
            None => {
                self.group_map.insert(
                    invoked_group_name,
                    vec![(comp_name, calyx_ir::Guard::True)],
                );
            }
        }
        // build a wrapper group
        let mut builder = ir::Builder::new(comp, sigs);
        let wrapper_group = builder.add_group("instrumentation_wrapper");
        let probe_cell_name = format!(
            "{}__{}__{}_probe",
            invoked_group_name,
            wrapper_group.borrow().name(),
            comp_name // wrapper_group.borrow().name()
        );
        let probe_cell =
            builder.add_primitive(probe_cell_name, "std_wire", &[1]);
        probe_cell.borrow_mut().add_attribute(BoolAttr::Control, 1);
        probe_cell
            .borrow_mut()
            .add_attribute(BoolAttr::Protected, 1);
        let one = builder.add_constant(1, 1);
        wrapper_group.borrow().get("done");
        // there is probably a better way to do this
        let start_invoked_group: ir::Assignment<Nothing> = builder
            .build_assignment(
                s.group.borrow().get("go"),
                one.borrow().get("out"),
                calyx_ir::Guard::True,
            );
        wrapper_group
            .borrow_mut()
            .assignments
            .push(start_invoked_group);
        let probe_asgn: ir::Assignment<Nothing> = builder.build_assignment(
            probe_cell.borrow().get("in"),
            one.borrow().get("out"),
            calyx_ir::Guard::True,
        );
        wrapper_group.borrow_mut().assignments.push(probe_asgn);
        let wrapper_done: ir::Assignment<Nothing> = builder.build_assignment(
            wrapper_group.borrow().get("done"),
            s.group.borrow().get("done"),
            calyx_ir::Guard::True,
        );
        wrapper_group.borrow_mut().assignments.push(wrapper_done);
        // TODO: need to replace the invocation of the original group with the wrapper group
        let en = ir::Control::enable(wrapper_group);
        Ok(Action::change(en)) // need to call Action::change() to swap out
    }

    fn finish(
        &mut self,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        // return
        Ok(Action::Stop)
    }
}
