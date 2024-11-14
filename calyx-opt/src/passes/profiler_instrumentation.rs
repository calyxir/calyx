use std::collections::HashMap;

use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, BoolAttr, Guard, Nothing};
use calyx_utils::CalyxResult;

/// Adds probe wires to each group to detect when a group is active.
/// Used by the profiler.
pub struct ProfilerInstrumentation {}

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
        Ok(ProfilerInstrumentation {})
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
        let mut structural_enable_map: HashMap<
            ir::Id,
            Vec<(ir::Id, ir::Guard<Nothing>)>,
        > = HashMap::new();
        let group_names = comp
            .groups
            .iter()
            .map(|group| group.borrow().name())
            .collect::<Vec<_>>();
        // iterate and check for structural enables
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
                        match structural_enable_map.get_mut(&invoked_group_name)
                        {
                            Some(vec_ref) => {
                                vec_ref.push((group.name(), combined_guard))
                            }
                            None => {
                                structural_enable_map.insert(
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
        // build probe and assignments for every group + all structural invokes
        let mut builder = ir::Builder::new(comp, sigs);
        let one = builder.add_constant(1, 1);
        let mut group_name_assign_and_cell = Vec::with_capacity(acc);
        {
            // probe and assignments for group (this group is currently active)
            // FIXME: probably best to remove the code clone by extracting this out into a different function?
            for group_name in group_names.into_iter() {
                // store group and component name (differentiate between groups of the same name under different components)
                let name = format!("{}__{}_probe_group", group_name, comp_name);
                let probe_cell = builder.add_primitive(name, "std_wire", &[1]);
                // let asgn: [ir::Assignment<ir::Nothing>; 1] = build_assignments!(
                //     builder;
                //     inst_cell["in"] = ? one["out"];
                // );
                let probe_asgn: ir::Assignment<Nothing> = builder
                    .build_assignment(
                        probe_cell.borrow().get("in"),
                        one.borrow().get("out"),
                        Guard::True,
                    );
                // the probes should be @control because they should have value 0 whenever the corresponding group is not active.
                probe_cell.borrow_mut().add_attribute(BoolAttr::Control, 1);
                probe_cell
                    .borrow_mut()
                    .add_attribute(BoolAttr::Protected, 1);
                group_name_assign_and_cell
                    .push((group_name, probe_asgn, probe_cell));
            }
            // probe and assignments for structural enables (this group is structurally enabling a child group)
            for (invoked_group_name, parent_groups) in
                structural_enable_map.iter()
            {
                for (parent_group, guard) in parent_groups.iter() {
                    let probe_cell_name = format!(
                        "{}__{}__{}_probe_se",
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
}
