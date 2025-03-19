use std::collections::HashMap;

use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{
    self as ir, BoolAttr, Guard, Id, Nothing, NumAttr, StaticTiming,
};
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
        let delimiter = "___";
        let mut acc = 0;
        let comp_name = comp.name;
        let mut structural_enable_map: HashMap<
            Id,
            Vec<(Id, ir::Guard<Nothing>)>,
        > = HashMap::new();
        // groups to cells (from non-primitive components) that they invoked
        let mut cell_invoke_map: HashMap<Id, Vec<Id>> = HashMap::new();
        // groups to primitives that they invoked
        let mut primitive_invoke_map: HashMap<
            Id,
            Vec<(Id, ir::Guard<Nothing>)>,
        > = HashMap::new();
        // child_group --> [(parent_group, Guard)]
        let group_names = comp
            .groups
            .iter()
            .map(|group| group.borrow().name())
            .collect::<Vec<_>>();
        let static_group_names = comp
            .static_groups
            .iter()
            .map(|group| group.borrow().name())
            .collect::<Vec<_>>();
        // Dynamic groups: iterate and check for structural enables and for cell invokes
        for group_ref in comp.groups.iter() {
            let group = &group_ref.borrow();
            let mut primitive_vec: Vec<(Id, ir::Guard<Nothing>)> = Vec::new();
            for assigment_ref in group.assignments.iter() {
                let dst_borrow = assigment_ref.dst.borrow();
                if let ir::PortParent::Group(parent_group_ref) =
                    &dst_borrow.parent
                {
                    if dst_borrow.name == "go" {
                        // found an invocation of go
                        let invoked_group_name =
                            parent_group_ref.upgrade().borrow().name();
                        let guard = *(assigment_ref.guard.clone());
                        match structural_enable_map.get_mut(&invoked_group_name)
                        {
                            Some(vec_ref) => {
                                vec_ref.push((group.name(), guard))
                            }
                            None => {
                                structural_enable_map.insert(
                                    invoked_group_name,
                                    vec![(group.name(), guard)],
                                );
                            }
                        }
                        acc += 1; // really sad hack
                    }
                }
                if let ir::PortParent::Cell(cell_ref) = &dst_borrow.parent {
                    match cell_ref.upgrade().borrow().prototype.clone() {
                        calyx_ir::CellType::Primitive {
                            name: _,
                            param_binding: _,
                            is_comb,
                            latency: _,
                        } => {
                            let cell_name = cell_ref.upgrade().borrow().name();
                            // don't need to profile for combinational primitives, and if the port isn't a go port.
                            if !is_comb & dst_borrow.has_attribute(NumAttr::Go)
                            {
                                let guard = Guard::and(
                                    *(assigment_ref.guard.clone()),
                                    Guard::port(ir::rrc(
                                        assigment_ref.src.borrow().clone(),
                                    )),
                                );
                                primitive_vec.push((cell_name, guard));
                            }
                        }
                        calyx_ir::CellType::Component { name: _ } => {
                            if dst_borrow.name == "go" {
                                let cell_name =
                                    cell_ref.upgrade().borrow().name();
                                match cell_invoke_map.get_mut(&group.name()) {
                                    Some(vec_ref) => {
                                        vec_ref.push(cell_name);
                                    }
                                    None => {
                                        cell_invoke_map.insert(
                                            group.name(),
                                            vec![cell_name],
                                        );
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            primitive_invoke_map
                .insert(group_ref.borrow().name(), primitive_vec);
        }
        // build probe and assignments for every group (dynamic and static) + all structural invokes
        let mut builder = ir::Builder::new(comp, sigs);
        let one = builder.add_constant(1, 1);
        let mut group_name_assign_and_cell = Vec::with_capacity(acc);
        let mut static_group_name_assign_and_cell =
            Vec::with_capacity(static_group_names.len()); // TODO: adjust when we figure out how to get primitives w/in static groups
        {
            // Static: probe and assignments for group (this group is currently active)
            for static_group_name in static_group_names.into_iter() {
                // store group and component name (differentiate between groups of the same name under different components)
                let name = format!(
                    "{}{}{}_group_probe",
                    static_group_name, delimiter, comp_name
                );
                let probe_cell = builder.add_primitive(name, "std_wire", &[1]);
                let probe_asgn: ir::Assignment<StaticTiming> = builder
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
                static_group_name_assign_and_cell.push((
                    static_group_name,
                    probe_asgn,
                    probe_cell,
                ));
            }
            // Dynamic: probe and assignments for group (this group is currently active)
            for group_name in group_names.into_iter() {
                // store group and component name (differentiate between groups of the same name under different components)
                let name = format!(
                    "{}{}{}_group_probe",
                    group_name, delimiter, comp_name
                );
                let probe_cell = builder.add_primitive(name, "std_wire", &[1]);
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
            // probe and assignments for primitive invocations (this group is activating a primitive)
            for (group, primitive_invs) in primitive_invoke_map.iter() {
                for (primitive_cell_name, guard) in primitive_invs.iter() {
                    let probe_cell_name = format!(
                        "{}{}{}{}{}_primitive_probe",
                        primitive_cell_name,
                        delimiter,
                        group,
                        delimiter,
                        comp_name
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
                    let probe_asgn: ir::Assignment<Nothing> = builder
                        .build_assignment(
                            probe_cell.borrow().get("in"),
                            one.borrow().get("out"),
                            guard.clone(),
                        );
                    group_name_assign_and_cell
                        .push((*group, probe_asgn, probe_cell));
                }
            }
            // probe and assignments for structural enables (this group is structurally enabling a child group)
            for (invoked_group_name, parent_groups) in
                structural_enable_map.iter()
            {
                for (parent_group, guard) in parent_groups.iter() {
                    let probe_cell_name = format!(
                        "{}{}{}{}{}_se_probe",
                        invoked_group_name,
                        delimiter,
                        parent_group,
                        delimiter,
                        comp_name
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
                    let probe_asgn: ir::Assignment<Nothing> = builder
                        .build_assignment(
                            probe_cell.borrow().get("in"),
                            one.borrow().get("out"),
                            guard.clone(),
                        );
                    group_name_assign_and_cell.push((
                        *parent_group,
                        probe_asgn,
                        probe_cell,
                    ));
                }
            }
            // probe cell and assignments for structural cell invocations (the group is structurally invoking a cell.)
            for (invoker_group, invoked_cells) in cell_invoke_map.iter() {
                for invoked_cell in invoked_cells {
                    let probe_cell_name = format!(
                        "{}{}{}{}{}_cell_probe",
                        invoked_cell,
                        delimiter,
                        invoker_group,
                        delimiter,
                        comp_name
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
                    // NOTE: this probe is active for the duration of the whole group. Hence, it may be active even when the cell itself is inactive.
                    let probe_asgn: ir::Assignment<Nothing> = builder
                        .build_assignment(
                            probe_cell.borrow().get("in"),
                            one.borrow().get("out"),
                            Guard::True,
                        );
                    group_name_assign_and_cell.push((
                        *invoker_group,
                        probe_asgn,
                        probe_cell,
                    ));
                }
            }
        }
        // Dynamic: Add created assignments to each group
        for group in comp.groups.iter() {
            for (group_name, asgn, cell) in group_name_assign_and_cell.iter() {
                if group.borrow().name() == group_name {
                    group.borrow_mut().assignments.push(asgn.clone());
                    comp.cells.add(cell.to_owned());
                }
            }
        }
        // Static: Add created assignments to each group
        for static_group in comp.static_groups.iter() {
            for (static_group_name, asgn, cell) in
                static_group_name_assign_and_cell.iter()
            {
                if static_group.borrow().name() == static_group_name {
                    static_group.borrow_mut().assignments.push(asgn.clone());
                    comp.cells.add(cell.to_owned());
                }
            }
        }
        Ok(Action::Continue)
    }
}
